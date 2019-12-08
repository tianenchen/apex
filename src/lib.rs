extern crate log as rlog;
pub mod node;
pub mod net;
pub mod log;
pub mod storage;
pub mod common;

#[cfg(test)]
mod tests{
    use crate::node::Peer;
    use super::*;
    use net::Letter;
    use async_std::{
        net::TcpStream,
        prelude::*,
        task,
    };
    use bincode::deserialize;
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    #[test]
    fn cluster_test() {
        init();
        std::thread::spawn(move || {
            task::block_on(async {
                let addr = "127.0.0.1:8080".to_string();
                let peers = "127.0.0.1:8081,127.0.0.1:8082".to_string();
                let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
                task::spawn(async move{
                    node::bootstrap(&addr.clone(),peers).await.unwrap();
                }).await;
            });
        });
        std::thread::spawn(move || {
            task::block_on(async {
                let addr = "127.0.0.1:8081".to_string();
                let peers = "127.0.0.1:8080,127.0.0.1:8082".to_string();
                let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
                task::spawn(async move{
                    node::bootstrap(&addr.clone(),peers).await.unwrap();
                }).await;
            });
        });
        std::thread::spawn(move || {
            task::block_on(async {
                let addr = "127.0.0.1:8082".to_string();
                let peers = "127.0.0.1:8080,127.0.0.1:8081".to_string();
                let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
                task::spawn(async move{
                    node::bootstrap(&addr.clone(),peers).await.unwrap();
                }).await;
            });
        });
        loop{}
    }

    #[test]
    fn server1() {
        init();
        task::block_on(async {
            let addr = "127.0.0.1:8080".to_string();
            let peers = "127.0.0.1:8081,127.0.0.1:8082".to_string();
            let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
            task::spawn(async move{
                node::bootstrap(&addr.clone(),peers).await.unwrap();
            }).await;
        });
        loop{}
    }

    #[test]
    fn server2() {
        init();
        task::block_on(async {
            let addr = "127.0.0.1:8081".to_string();
            let peers = "127.0.0.1:8080,127.0.0.1:8082".to_string();
            let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
            task::spawn(async move{
                node::bootstrap(&addr.clone(),peers).await.unwrap();
            }).await;
        });
        loop{}
    }

    #[test]
    fn server3() {
        init();
        task::block_on(async {
            let addr = "127.0.0.1:8082".to_string();
            let peers = "127.0.0.1:8080,127.0.0.1:8081".to_string();
            let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
            task::spawn(async move{
                node::bootstrap(&addr.clone(),peers).await.unwrap();
            }).await;
        });
        loop{}
    }
    #[test]
    fn client(){
        init();
        task::block_on(async move{
            let put = common::Command::PUT(common::K::from(b"hello".to_vec()),common::V::from(b"world".to_vec()));
            let put_letter:Vec<u8> = Letter::Command(put).into();
            let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            stream.write_all(&put_letter[..]).await.unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            let letter :Letter = deserialize(&buf[..]).unwrap();
            assert_eq!(letter,Letter::Reply(net::Reply::Success(None)));

            // let get = common::Command::GET(common::K::from(b"hello".to_vec()));
            // let get_letter:Vec<u8> = Letter::Command(get).into();
            // let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            // stream.write_all(&get_letter[..]).await.unwrap();
            // stream.shutdown(std::net::Shutdown::Write).unwrap();
            // let mut buf = Vec::new();
            // stream.read_to_end(&mut buf).await.unwrap();
            // let letter :Letter = deserialize(&buf[..]).unwrap();
            // assert_eq!(letter,Letter::Reply(net::Reply::Success(Some(common::V::from(b"world".to_vec())))));
        });
        // let body = serialize(&command);
        // let req = net::Request::new(net::RequestType::Message(command),body);
    }
}

