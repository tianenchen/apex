use std::cell::RefCell;
use futures::{channel::mpsc, select, FutureExt, SinkExt};
use std::time::Duration;
use async_std::{
    io,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    task,
};
use crate::log::RaftLog;
use crate::storage::StateMachine;
use crate::net::{Request,VoteRequest,RequestType};

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Peers = Vec<RefCell<PeerServer>>; 

#[derive(Debug,PartialEq)]
enum NodeState{
    FOLLOWER, CANDIDATE, LEADER
}

#[derive(Debug,PartialEq)]
pub struct RaftNode{
    state : NodeState,
    server_id : String,
    leader_id : Option<String>,
    current_term : u64,
    voted_for : Option<String>,
    peers : Peers,
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

    pub fn reset(&mut self){
        self.vote_granted = false;
    }
}

impl RaftNode {

    fn new(peers :Peers,server_id :String,state_machine :StateMachine)->Self{
        let last_index = state_machine.get_last_index();
        RaftNode{
            state:NodeState::FOLLOWER,
            leader_id:None,
            current_term:0,
            voted_for:None,
            logs: RaftLog::new(last_index),
            peers,
            server_id,
            state_machine,
        }
    }

    async fn receive(mut self,mut events: Receiver<Request>){
        self.pre_vote().await.unwrap();
        self.request_vote().await;
        let events = io::timeout(Duration::from_nanos(150),async {
            loop{
                match events.next().await{
                    Some(event)=>{
                        return Ok(event);
                    },
                    None=>continue,
                }
            }
        }).await;
        match events{
            Ok(events)=>self.handle_request(events).await,
            Err(_)=>self.timeout().await,
        }
    }

    async fn handle_request(self,req:Request){
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

    async fn timeout(self){
        println!("timeout");
    }

    async fn pre_vote(&mut self)->Result<()>{
        if self.state==NodeState::LEADER{
            return Err("during leader election timer, this should not happen!".into());
        }
        self.current_term+=1;
        self.voted_for=Some((&self.server_id).to_string());
        self.state=NodeState::CANDIDATE;
        Ok(())
    }

    async fn request_vote(&mut self){
        let request = VoteRequest::new(self.current_term, &self.server_id,self.logs.get_last_index(),self.logs.get_last_term());
        self.peers.iter().for_each(|i|{
            let peer = i.borrow_mut();
            peer.reset();
            async {
                request.send(&peer.end_point).await;
                Ok(())
            };
        });
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

pub async fn bootstrap(addr :&str, peers : Peers) -> Result<()>{
    let listener = TcpListener::bind(addr).await?;
    let raft_node = RaftNode::new(peers,addr.to_string(),StateMachine::new());
    let (producer, consumer) = mpsc::unbounded();
    let consumer = task::spawn(raft_node.receive(consumer));
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