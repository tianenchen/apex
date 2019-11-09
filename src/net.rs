use serde::{Serialize, Deserialize};
use bincode::{deserialize, serialize};
use async_std::{net::TcpStream, prelude::*};
use crate::log::LogEntry;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug)]
pub enum RequestType{
    Vote,
    AppendEntries,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request{
    pub rtype:RequestType,
    body:Vec<u8>,
}

impl Request{
    fn new(rtype:RequestType,body:Vec<u8>)->Self{
        Request{ rtype,body }
    }

    pub fn parse(package : &[u8])->Result<Self>{
        Ok(deserialize(&package[..])?)
    }

    async fn send(self,peer :&str)->Result<Vec<u8>>{
        let mut stream = TcpStream::connect(peer).await?;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteRequest{
    term:u64,
    candidate_id:String,
    last_log_index:u64,
    last_log_term:u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteResponse{
    term:u64,
    vote_granted:bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesRequest{
    term :u64,
    leader_id:String,
    prev_log_index:u64,
    prev_log_term:u64,
    entries:Vec<LogEntry>,
    leader_commit:u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesResponse{
    term :u64,
    success :bool,
}

impl AppendEntriesRequest{
    pub fn new(term :u64,leader_id: &str,prev_log_index:u64,prev_log_term:u64,entries:Vec<LogEntry>,leader_commit:u8) -> Self{
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




