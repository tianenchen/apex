use futures::{channel::mpsc,SinkExt,select,FutureExt};
use std::time::Duration;
use std::io::{ Error , ErrorKind::Interrupted };
use async_std::{
    io,
    net::{TcpListener, TcpStream},
    prelude::*,
    task,
    stream,
};
use crate::log::RaftLog;
use crate::storage::StateMachine;
use crate::net::{Request,VoteRequest,AppendEntriesRequest,RequestType};

const MAX_LOG_ENTRIES_PER_REQUEST :u64 = 100;

const HEART_BEAT_TIMEOUT_MS :u64 = 500;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug,PartialEq)]
enum NodeState{
    FOLLOWER, CANDIDATE, LEADER
}

#[derive(Debug)]
pub struct RaftNode{
    state : NodeState,
    server_id : String,
    leader_id : Option<String>,
    commit_index : u64,
    current_term : u64,
    voted_for : Option<String>,
    peers : Vec<PeerServer>,
    logs :RaftLog,
    state_machine :StateMachine,
}

#[derive(Debug,Hash,PartialEq,Eq)]
pub struct PeerServer{
    end_point:String,
    vote_granted:bool,
    next_index:u64,
    match_index:u64,
}

impl PeerServer{
    pub fn new(end_point:&str)->Self{
        PeerServer{
            end_point:end_point.to_string(),
            vote_granted:false,
            next_index:1,
            match_index:0,
        }
    }
}

impl RaftNode {

    fn new(peers :Vec<PeerServer>,server_id :String,state_machine :StateMachine)->Self{
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

    async fn receive(&mut self,events: &mut Receiver<Request>){
        loop{
            let event = io::timeout(Duration::from_millis(300),async {
                events.next().await.ok_or(Error::from(Interrupted))
            }).await;
            match event{
                Ok(event)=>self.handle_request(event).await,
                Err(_)=>{
                    //out of time slice
                    while io::timeout(Duration::from_millis(1000),self.vote()).await.is_err() {}
                    //finally , become leader or follower
                    match self.state{
                        NodeState::LEADER => {
                            self.become_leader();
                            self.serve(events).await;
                        },
                        _ => continue,
                    }
                },
            }
        }
    }

    async fn serve(&mut self , events: &mut Receiver<Request>){
        let gap = Duration::from_millis(HEART_BEAT_TIMEOUT_MS);
        let mut interval = stream::interval(gap);
        loop{
            select!{
                heartbeat = interval.next().fuse() =>{
                    self.append_entries().await;
                },
                append_entries = events.next().fuse() =>{
                    //todo deal client request
                    self.append_entries().await;
                    interval = stream::interval(gap);
                },
                complete => {
                    
                }
            }
        }
    }

    async fn handle_request(&mut self,req:Request){
        match req.rtype{
            RequestType::Vote=>{
                println!("vote");
                //todo
            },
            RequestType::AppendEntries=>{
                println!("append entriess");
                //todo
            },
        }
    }

    async fn vote(&mut self)->io::Result<()>{
        self.current_term+=1;
        self.voted_for=Some((&self.server_id).to_string());
        self.state=NodeState::CANDIDATE;
        let (tx, mut rx) = mpsc::unbounded();
        let request = VoteRequest::new(self.current_term, &self.server_id,self.logs.get_last_index(),self.logs.get_last_term());
        for peer in &mut self.peers[..]{
            peer.vote_granted = false;
            let end_point = peer.end_point.clone();
            println!("send request : {}",end_point);
            let mut tx = tx.clone();
            let request = request.clone();
            task::spawn(async move{
                tx.send(request.send(&end_point).await).await.unwrap();
            });
        }
        let mut vote_granted_num = 0;
        while let Some(resp) = rx.next().await{
            match resp{
                Ok(resp) if resp.term>self.current_term =>{
                    self.state = NodeState::FOLLOWER;
                    return Ok(())
                },
                Ok(resp) if resp.vote_granted =>{
                    vote_granted_num+=1;
                    if vote_granted_num > (&self.peers.len()+1)/2{
                        self.state = NodeState::LEADER;
                        return Ok(())
                    }
                },
                Ok(_) => {
                    println!("Let me down");
                },
                Err(_)=>{
                    println!("oops , request fail");
                }
            }
        }
        Err(Error::from(Interrupted))
    }

    fn is_leader(&self)->bool{
        match &self.leader_id{
            Some(leader_id) if leader_id == &self.server_id => true,
            _ => false,
        }
    }

    fn become_leader(&mut self){
        self.leader_id = Some((&self.server_id).to_string());
        self.state = NodeState::LEADER;
        let last_index = self.logs.get_last_index();
        self.peers.iter_mut().for_each(|peer|{
            peer.next_index =  last_index+1;
        });
    }

    async fn append_entries(&mut self){
        let (tx , mut rx) = mpsc::unbounded();
        for (i,peer) in self.peers.iter_mut().enumerate(){
            let prev_log_index = peer.next_index - 1;
            let last_index = std::cmp::min(self.logs.get_last_index(),MAX_LOG_ENTRIES_PER_REQUEST+prev_log_index);
            let prev_log_term = self.logs.get_log_term(prev_log_index);
            let entrys = self.logs.get_log_entrys(peer.next_index,last_index);
            let end_point = peer.end_point.clone();
            let server_id = self.server_id.clone();
            let term = self.current_term;
            let leader_commit = self.commit_index;
            let mut tx = tx.clone();
            task::spawn(async move{
                let request = AppendEntriesRequest::new(term,&server_id,prev_log_index, prev_log_term,entrys,leader_commit);
                tx.send((i,request.send(&end_point).await)).await.unwrap();
            });
        }
        while let Some((index,resp)) = rx.next().await{
            match resp{
                Ok(res)=>{
                    let last_index = std::cmp::min(self.logs.get_last_index(),MAX_LOG_ENTRIES_PER_REQUEST+self.peers[index].next_index-1);
                    if res.success{
                        self.peers[index].match_index = last_index;
                        self.peers[index].next_index = last_index + 1;
                    }
                    else{
                        self.peers[index].next_index -=  1;
                    }
                },
                Err(_)=>{
                    println!("oops , request fail");
                },
            }
        }
    }

    async fn handle_connection(mut producer: Sender<Request> ,mut stream : TcpStream)->Result<()>{
        //此处处理接受到的请求
        //所有的外部调用由这里处理
        //可能存在的请求有
        //1. 投票请求
        //2. 日志追加请求
        //3. 外部客户端查询状态请求
        let mut buf : [u8;256] = [0;256];
        if let Ok(_n) = stream.read(&mut buf).await {
            // producer.send(RequestType::AppendEntries).await.unwrap();
        }
        Ok(())
    }
}

pub async fn bootstrap(addr :&str, peers : Vec<PeerServer>) -> Result<()>{
    let listener = TcpListener::bind(addr).await?;
    let (producer, mut consumer) = mpsc::unbounded();
    let mut raft_node = RaftNode::new(peers,addr.to_string(),StateMachine::new());
    let consumer = task::spawn(async move{
        raft_node.receive(&mut consumer).await;
    });
    let mut incoming = listener.incoming();
    while let Some(Ok(stream)) = incoming.next().await{
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(RaftNode::handle_connection(producer.clone(),stream));
    }
    drop(producer);
    consumer.await;
    Ok(())
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{}", e)
        }
    })
}