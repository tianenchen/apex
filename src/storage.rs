use std::collections::HashMap;
use crate::common::{Command,K,V};
use crate::log::LogEntry;

#[derive(Default)]
pub struct Snapshot{
    last_index:u64,
    last_term:u64,
}

impl Snapshot{
    pub fn set_last_index(&mut self,last_index :u64){
        self.last_index = last_index;
    }
    pub fn set_last_term(&mut self,last_term :u64){
        self.last_term = last_term;
    }
}


pub trait StateMachine{
    fn get_last_index(&self)->u64;
    fn get_last_term(&self)->u64;
    fn query(&self,key :K)->Option<V>;
    fn apply(&mut self,logs :&[LogEntry]);
    fn snapshot(&self)->Snapshot;
    fn take_a_snapshot(&mut self,snapshot :Snapshot);
    fn restore_snapshot(&mut self,snapshot :Snapshot)->std::io::Result<()>;
}


#[derive(Debug,PartialEq)]
pub struct MemKVStateMachine{
    last_index:u64,
    last_term:u64,
    state :HashMap<K,V>
}

impl Default for MemKVStateMachine{
    fn default() -> Self{
        MemKVStateMachine{
            last_index:0,
            last_term:0,
            state:HashMap::new()
        }
    }
}

#[allow(unused_variables)]
impl StateMachine for MemKVStateMachine{

    fn get_last_index(&self)->u64{
        self.last_index
    }

    fn get_last_term(&self)->u64{
        self.last_term
    }

    fn query(&self,key :K)->Option<V>{
        match self.state.get(&key){
            Some(value) => Some(value.clone()),
            None => None,
        }
    }

    fn apply(&mut self,logs :&[LogEntry]){
        logs.iter().for_each(|log|{
            match &log.command{
                Command::PUT(k,v)=>self.state.insert(k.clone(),v.clone()),
                Command::DELETE(k)=>self.state.remove(&k),
                _=>None,
            };
        });
    }
    fn snapshot(&self)->Snapshot{
        Snapshot{
            last_index:self.last_index,
            last_term:self.last_term,
        }
    }

    fn take_a_snapshot(&mut self,snapshot :Snapshot){
        if snapshot.last_index>self.get_last_index()&&snapshot.last_term>self.last_term {
            self.last_index = snapshot.last_index;
            self.last_term = snapshot.last_term;
        }
    }

    fn restore_snapshot(&mut self,_snapshot :Snapshot)->std::io::Result<()>{
        Ok(())
    }
}

// mod tests{
//     #[test]
//     fn test1() {
//         use super::*;
//         use super::LogEntry;
//         let mut state_mechine = MemKVStateMachine::default();
//         let put = Command::PUT(K::from(b"hello".to_vec()), V::from(b"world".to_vec()));
//         let logs = vec![LogEntry::new(1,1,put)];
//         state_mechine.apply(&logs);
//         let res = state_mechine.query(K::from(b"hello".to_vec()));
//         assert_eq!(res.unwrap(),V::from(b"world".to_vec()));
//     }
// }


