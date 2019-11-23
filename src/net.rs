use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};
use async_std::{io,net::TcpStream, prelude::*};
use crate::log::LogEntry;
use crate::common::Result;

#[derive(Serialize, Deserialize, Debug)]
pub enum RequestType{
    Vote,
    AppendEntries,
    Message,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request{
    pub rtype:RequestType,
    pub body:Vec<u8>,
}

impl Request{
    fn new(rtype:RequestType,body:Vec<u8>)->Self{
        Request{ rtype,body }
    }

    pub fn parse(package : &[u8])->Result<Self>{
        Ok(deserialize(&package[..])?)
    }

    pub fn to_vote_request(body :&[u8])->Result<VoteRequest>{
        Ok(deserialize(&body[..])?)
    }
    
    pub fn to_append_request(body :&[u8])->Result<AppendEntriesRequest>{
        Ok(deserialize(&body[..])?)
    }

    async fn send(self,peer :&str)->Result<Vec<u8>>{
        println!("sending ");
        let mut stream = io::timeout(std::time::Duration::from_millis(100),async {
            TcpStream::connect(peer).await
        }).await?;
        // let mut stream = TcpStream::connect(peer).await?;
        let req = serialize(&self)?;
        stream.write_all(&req[..]).await?;
        let mut buf = vec![];
        stream.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn vote(vote :&VoteRequest,peer :&str)->Result<VoteResponse>{
        let wrap = Self::new(RequestType::Vote, serialize(&vote)?);
        let resp = wrap.send(peer).await?;
        let resp : VoteResponse = deserialize(&resp[..])?;
        Ok(resp)
    }

    async fn append_entries(entries :&AppendEntriesRequest,peer :&str)->Result<AppendEntriesResponse>{
        let wrap = Self::new(RequestType::AppendEntries, serialize(&entries)?);
        let resp = wrap.send(peer).await?;
        let resp : AppendEntriesResponse = deserialize(&resp[..])?;
        Ok(resp)
    }
}

#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct VoteRequest{
    pub term:u64,
    pub candidate_id:String,
    pub last_log_index:u64,
    pub last_log_term:u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteResponse{
    pub term:u64,
    pub vote_granted:bool,
}

impl VoteResponse{
    pub fn new(term :u64,vote_granted :bool)->Vec<u8>{
        let resp = VoteResponse{
            term,vote_granted
        };
        serialize(&resp).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesRequest{
    pub term :u64,
    pub leader_id:String,
    pub prev_log_index:u64,
    pub prev_log_term:u64,
    pub entries:Option<Vec<LogEntry>>,
    pub leader_commit:u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesResponse{
    term :u64,
    pub success :bool,
}

impl AppendEntriesResponse{
    pub fn new(term :u64,success :bool)->Vec<u8>{
        let resp = AppendEntriesResponse{
            term,success
        };
        serialize(&resp).unwrap()
    }
}

impl AppendEntriesRequest{
    pub fn new(term :u64,leader_id: &str,prev_log_index:u64,prev_log_term:u64,entries:Option<Vec<LogEntry>>,leader_commit:u64) -> Self{
        AppendEntriesRequest{
            term ,
            leader_id:leader_id.to_string(),
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }
    }

    pub async fn send(&self,peer :&str)->Result<AppendEntriesResponse>{
        Request::append_entries(&self,peer).await
    }
}

struct AppendEntriesRequestArgs{

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

    pub async fn send(&self,peer :&str)->Result<VoteResponse>{
        Request::vote(self,peer).await
    }
}




