use futures::{channel::mpsc,SinkExt,select,FutureExt};
use std::time::Duration;
use rand::{thread_rng, Rng};
use log::{info,debug};
use async_std::{
    io,
    net::{TcpListener, TcpStream},
    prelude::*,
    task,
    stream,
};
use crate::log::{RaftLog,LogEntry};
use crate::storage::{StateMachine,MemKVStateMachine};
use crate::net::{*,Reason::*};
use crate::common::{Result,Command,Error};

const MAX_LOG_ENTRIES_PER_REQUEST :u64 = 100;

const HEART_BEAT_TIMEOUT_MS :u64 = 50;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[derive(Debug,PartialEq)]
enum NodeState{
    FOLLOWER, CANDIDATE, LEADER
}

enum Signal{
    Degrade,
    Next,
    Block,
}

pub struct RaftNode<T:StateMachine>{
    state : NodeState,
    server_id : String,
    leader_id : Option<String>,
    commit_index : u64,
    current_term : u64,
    voted_for : Option<String>,
    peers : Vec<Peer>,
    logs :RaftLog,
    state_machine :T,
}

#[derive(Debug,Hash,PartialEq,Eq)]
pub struct Peer{
    end_point:String,
    vote_granted:bool,
    next_index:u64,
    match_index:u64,
}

impl Peer{
    pub fn new(end_point:&str)->Self{
        Peer{
            end_point:end_point.to_string(),
            vote_granted:false,
            next_index:1,
            match_index:0,
        }
    }
}

impl <T :StateMachine> RaftNode<T> {
    fn new(peers :Vec<Peer>,server_id :String, state_machine : T )->Self{
        let last_index = state_machine.get_last_index();
        RaftNode{
            state:NodeState::FOLLOWER,
            leader_id:None,
            commit_index:0,
            current_term:0,
            voted_for:None,
            logs: RaftLog::new(last_index),
            peers,
            server_id,
            state_machine,
        }
    }

    async fn receive(&mut self,events: &mut Receiver<Communication>){
        loop{
            let rng :u64 = thread_rng().gen_range(150, 300);
            let event = io::timeout(Duration::from_millis(rng),async {
                Ok(events.next().await)
            }).await;
            match event{
                Ok(Some(mut event))=>{
                    let _ = self.handle_request(&mut event).await.expect("receive msg fail !!!!");
                },
                _ =>{
                    if let Ok(()) = io::timeout(Duration::from_millis(rng-150),self.vote()).await{
                        match self.state{
                            NodeState::LEADER => {
                                self.become_leader();
                                self.serve(events).await;
                            },
                            _ => (),
                        }
                    }
                    self.voted_for = None;
                },
            };
        }
    }

    async fn serve(&mut self , events: &mut Receiver<Communication>){
        let gap = Duration::from_millis(HEART_BEAT_TIMEOUT_MS);
        let mut interval = stream::interval(gap);
        loop{
            let state = select!{
                heartbeat = interval.next().fuse() =>{
                    match heartbeat{
                        Some(heartbeat) => self.try_append_entries().await,
                        None => break,
                    }
                },
                request = events.next().fuse() =>{
                    match request{
                        Some(mut request) => {
                            let signal = self.handle_request(&mut request).await;
                            interval = stream::interval(gap);
                            signal
                        },
                        None => break,
                    }
                },
            };
            match state{
                Ok(signal) => {
                    match signal{
                        Signal::Next => {
                            let mut snapshot = self.state_machine.snapshot();
                            snapshot.set_last_index(self.commit_index);
                            snapshot.set_last_term(self.current_term);
                            self.state_machine.take_a_snapshot(snapshot);
                            self.logs.truncate(self.commit_index);
                        }
                        Signal::Degrade => break,
                        _ => panic!("This will never happen"),
                    }
                },
                Err(_) => continue,
            }
        }
        self.become_follower();
    }

    async fn handle_request(&mut self,comm:&mut Communication)->Result<Signal>{
        // info!("handle letter : {:?}",&comm.letter);
        let resp = match &comm.letter{
            Letter::VoteRequest(vote)=>{
                match vote{
                    vote if vote.term < self.current_term => Some(VoteResponse::new(self.current_term,false).as_letter()),
                    vote if (self.voted_for == None || self.voted_for == Some(vote.candidate_id.clone()))
                                &&vote.last_log_index<=self.logs.latest_log_index()
                                &&vote.last_log_term<=self.logs.latest_log_term() 
                     =>Some(VoteResponse::new(self.current_term,true).as_letter()),
                    vote if vote.term > self.current_term && self.is_leader() => None,
                    _=>Some(VoteResponse::new(self.current_term,false).as_letter()),
                }
            },
            Letter::AppendEntriesRequest(append_entries)=>{
                match append_entries {
                    append_entries if append_entries.term < self.current_term => Some(AppendEntriesResponse::new(self.current_term,false).as_letter()),
                    append_entries if append_entries.term > self.current_term && self.is_leader() => None,
                    append_entries => {
                        let entry = self.logs.entry(append_entries.prev_log_index);
                        let leader_commit = append_entries.leader_commit;
                        match entry{
                            Some(entry) if self.logs.latest_log_term() != entry.term => Some(AppendEntriesResponse::new(self.current_term,false).as_letter()),
                            _=> {
                                let last_index = match append_entries.entries.last(){
                                    Some(last) => last.index,
                                    None => self.logs.latest_log_index(),
                                };
                                self.logs.append(append_entries.prev_log_index,append_entries.entries.clone());
                                if leader_commit > self.commit_index{
                                    let next_index = std::cmp::min(leader_commit, last_index);
                                    self.state_machine.apply(self.logs.entries(self.commit_index+1,next_index));
                                    self.commit_index = next_index;
                                }
                                Some(AppendEntriesResponse::new(self.current_term,true).as_letter())
                            },
                        }
                    }
                }
            },
            Letter::Command(cmd)=>{
                let resp = match (self.is_leader(),cmd){
                    (true,Command::GET(k)) => Reply::Success(self.state_machine.query(k.clone())),
                    (true,_) => {
                        let index = self.logs.latest_log_index()+1;
                        let log = LogEntry::new(index, self.current_term,cmd.clone());
                        let next_index = self.logs.append(index,vec![log]);
                        match self.try_append_entries().await{
                            Ok(signal)=>{
                                match signal{
                                    Signal::Next=>{
                                        self.state_machine.apply(self.logs.entries(self.commit_index+1,next_index));
                                        Reply::Success(None)
                                    },
                                    _ => return Ok(Signal::Degrade)
                                }

                            },
                            Err(_)=>Reply::Fail(Timeout),
                        }
                    },
                    (false,_) => {
                        match &self.leader_id{
                            Some(leader_id) => Reply::Fail(Redirect(leader_id.clone())),
                            None => Reply::Fail(Unavailable),
                        }
                    },
                };
                Some(resp.as_letter())
            },
            _ => panic!("this should not happen"),
        };
        match resp{
            Some(resp) => {
                let _ = comm.reply(resp).await;
                Ok(Signal::Next)
            },
            None => Ok(Signal::Degrade)
        }
    }

    async fn vote(&mut self)->io::Result<()>{
        self.current_term+=1;
        info!("current term : {}", self.current_term );
        self.voted_for=Some((&self.server_id).to_string());
        self.state=NodeState::CANDIDATE;
        let (tx, mut rx) = mpsc::unbounded();
        let request = VoteRequest::new(self.current_term, &self.server_id,self.logs.latest_log_index(),self.logs.latest_log_term());
        for peer in &mut self.peers[..]{
            peer.vote_granted = false;
            let end_point = peer.end_point.clone();
            let mut tx = tx.clone();
            let request = request.clone();
            task::spawn(async move{
                let _ = tx.send(request.send(&end_point).await).await;
            });
        }
        let mut vote_granted_num = 1;
        while let Some(resp) = rx.next().await{
            match resp{
                Ok(resp) if resp.term>self.current_term =>{
                    self.state = NodeState::FOLLOWER;
                    info!("peer : {} , vote granted num : {}", self.server_id ,vote_granted_num );
                    return Ok(())
                },
                Ok(resp) if resp.vote_granted =>{
                    vote_granted_num+=1;
                    info!("peer : {} , vote granted num : {} , threshold : {}", self.server_id ,vote_granted_num,(&self.peers.len()+1)/2);
                    if vote_granted_num > (&self.peers.len()+1)/2{
                        self.state = NodeState::LEADER;
                        return Ok(())
                    }
                },
                Ok(_) => {
                    info!("Let me down");
                },
                Err(e)=>{
                    eprint!("{}",e);
                    info!("oops , request fail");
                }
            }
        }
        Ok(())
    }

    fn is_leader(&self)->bool{
        match &self.leader_id{
            Some(leader_id) if leader_id == &self.server_id => true,
            _ => false,
        }
    }

    fn become_leader(&mut self){
        info!("peer : {} become leader",self.server_id);
        self.leader_id = Some((&self.server_id).to_string());
        self.state = NodeState::LEADER;
        let last_index = self.logs.latest_log_index();
        self.peers.iter_mut().for_each(|peer|{
            peer.next_index =  last_index+1;
        });
    }

    fn become_follower(&mut self){
        self.leader_id = None;
        self.voted_for = None;
        self.state= NodeState::FOLLOWER;
    }

    async fn try_append_entries(&mut self)->Result<Signal>{
        io::timeout(Duration::from_millis(HEART_BEAT_TIMEOUT_MS),async {
            while let Ok(Signal::Block) =  self.append_entries().await {}
            Ok(Signal::Next)
        }).await.map_err(Error::from)
    }

    async fn append_entries(&mut self)->Result<Signal>{
        let (tx , mut rx) = mpsc::unbounded();
        for (i,peer) in self.peers.iter_mut().enumerate(){
            let prev_log_index = peer.next_index - 1;
            let last_index = std::cmp::min(self.logs.latest_log_index(),MAX_LOG_ENTRIES_PER_REQUEST+prev_log_index);
            let prev_log_term = self.logs.term(prev_log_index);
            let entries = self.logs.entries(peer.next_index,last_index)
                    .iter()
                    .map(|entry| entry.clone())
                    .collect::<Vec<LogEntry>>();
            let end_point = peer.end_point.clone();
            let server_id = self.server_id.clone();
            let term = self.current_term;
            let leader_commit = self.commit_index;
            let mut tx = tx.clone();
            task::spawn(async move{
                let request = AppendEntriesRequest::new(term,&server_id,prev_log_index, prev_log_term,entries,leader_commit);
                debug!("{:?}",request);
                let resp = request.send(&end_point).await;
                let _ = tx.send((i,resp)).await;
            });
        }
        let mut received_num = 1;
        while let Some((index,resp)) = rx.next().await{
            match resp{
                Ok(res)=>{
                    let last_index = std::cmp::min(self.logs.latest_log_index(),MAX_LOG_ENTRIES_PER_REQUEST+self.peers[index].next_index-1);
                    if res.success{
                        self.peers[index].match_index = last_index;
                        self.peers[index].next_index = last_index + 1;
                        received_num += 1;
                    }
                    else{
                        if res.term > self.current_term {
                            return Ok(Signal::Degrade)
                        }
                        self.peers[index].next_index -=  1;
                    }
                },
                Err(_)=>{
                    info!("oops , request fail");
                },
            }
        }
        if received_num > (&self.peers.len()+1)/2 {
            return Ok(Signal::Next);
        }
        else {
            return Ok(Signal::Block);
        }
    }
}

pub async fn bootstrap(addr :&str, peers : Vec<Peer>) -> Result<()>{
    let listener = TcpListener::bind(addr).await?;
    let (producer, mut consumer) = mpsc::unbounded();
    let mut raft_node = RaftNode::new(peers,addr.to_string(),MemKVStateMachine::default());
    let consumer = task::spawn(async move{
        raft_node.receive(&mut consumer).await;
    });
    let mut incoming = listener.incoming();
    while let Some(Ok(stream)) = incoming.next().await{
        info!("{} , Accepting from: {}",addr, stream.peer_addr()?);
        spawn_and_log_error(handle_connection(producer.clone(),stream));
    }
    drop(producer);
    consumer.await;
    Ok(())
}

async fn handle_connection(mut producer: Sender<Communication> ,mut stream : TcpStream)->Result<()>{
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read incomming stream error");
    let letter = Letter::from(&buf[..]);
    Ok(producer.send(Communication::new(letter,stream)).await?)
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprint!("{}", e)
        }
    })
}

mod tests{
}