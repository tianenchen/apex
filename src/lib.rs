extern crate log as rlog;
pub mod node;
pub mod net;
pub mod log;
pub mod storage;
pub mod common;

#[cfg(test)]
mod tests{
    use crate::node::Peer;
    use async_std::task;
    use super::*;
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
            let addr = "127.0.0.1:8081".to_string();
            let peers = "127.0.0.1:8080,127.0.0.1:8082".to_string();
            let peers :Vec<Peer> = peers.split(',').map(Peer::new).collect();
            task::spawn(async move{
                node::bootstrap(&addr.clone(),peers).await.unwrap();
            }).await;
        });
        loop{}
    }
}

