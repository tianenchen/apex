use crate::common::Command;
use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};
use async_std::{net::TcpStream, prelude::*};
use std::net::Shutdown;
use log::info;
use crate::log::LogEntry;
use crate::common::{Result,V};


#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Letter{

    VoteRequest(VoteRequest),

    AppendEntriesRequest(AppendEntriesRequest),

    Command(Command),

    VoteResponse(VoteResponse),

    AppendEntriesResponse(AppendEntriesResponse),

    Reply(Reply)

}

impl Letter{
    async fn send(self,peer :&str)->Result<Self>{
        let req = serialize(&self)?;
        let mut buf = Vec::new();
        let mut stream = TcpStream::connect(peer).await?;
        stream.write_all(&req[..]).await?;
        stream.shutdown(Shutdown::Write)?;
        stream.read_to_end(&mut buf).await?;
        let letter = deserialize(&buf)?;
        Ok(letter)
    }
}

impl From<&[u8]> for Letter {
    fn from(val: &[u8]) -> Letter {
        deserialize(val).expect("deserialize letter fail")
    }
}
impl Into<Vec<u8>> for Letter {
    fn into(self) -> Vec<u8> {
        serialize(&self).expect("serialize letter fail")
    }
}

#[derive(Debug)]
pub struct Communication{
    pub letter : Letter ,
    pub contact : TcpStream ,
}

impl Communication{
    pub fn new(letter : Letter , contact : TcpStream) -> Self{
        Communication{ letter , contact }
    }

    pub async fn reply(&mut self,reply : Letter) -> Result<()>{
        let reply : Vec<u8> = reply.into();
        self.contact.write_all(&reply[..]).await?;
        self.contact.shutdown(Shutdown::Both)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq ,Debug)]
pub enum Reply{
    Success(Option<V>),
    Fail(Reason)
}

impl Reply{
    pub fn as_letter(self)->Letter{
        Letter::Reply(self)
    }
}

#[derive(Serialize, Deserialize,PartialEq, Debug)]
pub enum Reason{
    Redirect(String),
    InvalidArgument,
    Timeout,
    Unavailable,
}

impl Reply{
    pub fn serialize(self)->Vec<u8>{
        serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize,PartialEq, Debug,Clone)]
pub struct VoteRequest{
    pub term:u64,
    pub candidate_id:String,
    pub last_log_index:u64,
    pub last_log_term:u64,
}

impl VoteRequest{
    pub fn new(term:u64,candidate_id :&str,last_log_index:u64,last_log_term:u64)->Self{
        VoteRequest{
            term,
            candidate_id:candidate_id.to_string(),
            last_log_index,
            last_log_term,
        }
    }

    pub async fn send(self,peer :&str)->Result<VoteResponse>{
        match Letter::VoteRequest(self).send(peer).await?{
            Letter::VoteResponse(resp) => Ok(resp),
            _ => panic!("this should not happen"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct VoteResponse{
    pub term:u64,
    pub vote_granted:bool,
}

impl VoteResponse{
    pub fn new (term :u64,vote_granted :bool) -> Self{
        VoteResponse{ term,vote_granted }
    }

    pub fn as_letter(self)->Letter{
        Letter::VoteResponse(self)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AppendEntriesRequest{
    pub term :u64,
    pub leader_id:String,
    pub prev_log_index:u64,
    pub prev_log_term:u64,
    pub entries:Vec<LogEntry>,
    pub leader_commit:u64,
}


impl AppendEntriesRequest{
    pub fn new(term :u64,leader_id: &str,prev_log_index:u64,prev_log_term:u64,entries:Vec<LogEntry>,leader_commit:u64) -> Self{
        AppendEntriesRequest{
            term ,
            leader_id:leader_id.to_string(),
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }
    }

    pub async fn send(self,peer :&str)->Result<AppendEntriesResponse>{
        match Letter::AppendEntriesRequest(self).send(peer).await?{
            Letter::AppendEntriesResponse(resp) => Ok(resp),
            _ => panic!("this should not happen"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct AppendEntriesResponse{
    term :u64,
    pub success :bool,
}

impl AppendEntriesResponse{
    pub fn new(term :u64,success :bool)->Self{
        AppendEntriesResponse{
            term,success
        }
    }
    pub fn as_letter(self)->Letter{
        Letter::AppendEntriesResponse(self)
    }
}

#[cfg(test)]
mod tests{
use super::*;
    use async_std::{
        io,
        net::{TcpListener, TcpStream},
        prelude::*,
        task,
        stream,
    };
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    fn simulation(){
        task::spawn(async {
            let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();
            let mut incoming = listener.incoming();
            while let Some(Ok(mut stream)) = incoming.next().await{
                task::spawn(async move {
                    let mut buf = Vec::new();
                    stream.read_to_end(&mut buf).await.expect("read stream error");
                    match Letter::from(&buf[..]){
                        Letter::VoteRequest(req)=>{
                            info!("{:?}",req);
                            let letter = VoteResponse::new(1,true).as_letter();
                            let mut communication = Communication::new(Letter::VoteRequest(req),stream);
                            communication.reply(letter).await.unwrap();
                        },
                        Letter::AppendEntriesRequest(req)=>{
                            info!("{:?}",req);
                            let letter = AppendEntriesResponse::new(1,true).as_letter();
                            let mut communication = Communication::new(Letter::AppendEntriesRequest(req),stream);
                            communication.reply(letter).await.unwrap();
                        },
                        Letter::Command(cmd)=>{
                            info!("{:?}",cmd);
                        },
                        Letter::VoteResponse(resp)=>{
                            info!("{:?}",resp);
                        },
                        Letter::AppendEntriesResponse(resp)=>{
                            info!("{:?}",resp);
                        },
                        Letter::Reply(rep)=>{
                            info!("{:?}",rep);
                        },
                    }
                });
            }
        });
    }

    #[test]
    fn vote_test() {
        init();
        simulation();
        task::block_on(async move{
            let req = VoteRequest::new(1,"127.0.0.1:8080",1,1);
            let resp = req.send("127.0.0.1:9999").await.unwrap();
            assert_eq!(resp,VoteResponse::new(1,true));
        });
    }

    #[test]
    fn append_test() {
        init();
        simulation();
        task::block_on(async move{
            let req = AppendEntriesRequest::new(1,"127.0.0.1:8080",0,0,Vec::new(),0);
            let resp = req.send("127.0.0.1:9999").await.unwrap();
            assert_eq!(resp,AppendEntriesResponse::new(1,true));
        });
    }

    #[test]
    fn empty_pkg_test() {
        init();
        simulation();
        task::block_on(async move{
            let mut stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
            // task::sleep(std::time::Duration::from_secs(1)).await;
        });
    }
}