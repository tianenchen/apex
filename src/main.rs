use crate::node::PeerServer;
use std::env;
use async_std::task;

mod node;
mod net;
mod log;
mod storage;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main(){
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let peers = env::args().nth(2).unwrap_or_else(|| "127.0.0.1:8081,127.0.0.1:8082".to_string());
    let peers :Vec<PeerServer> = peers.split(',').map(PeerServer::new).collect();
    task::block_on(node::bootstrap(&addr,peers)).expect("oops");
}

