use std::collections::HashMap;

#[derive(Default)]
pub struct Snapshot{
    last_index:u64,
    last_term:u64,
}


pub trait StateMachine{
    fn get_last_index(&self)->u64;
    fn get_last_term(&self)->u64;
    fn query(&self,key :&[u8])->Option<Vec<u8>>;
    fn apply(&mut self,key :Vec<u8>,value :Vec<u8>)->bool;
    fn snapshot(&mut self)->Snapshot;
    fn restore_snapshot(&mut self,snapshot :Snapshot)->std::io::Result<()>;
}


#[derive(Debug,PartialEq)]
pub struct MemKVStateMachine{
    last_index:u64,
    last_term:u64,
    state :HashMap<Vec<u8>,Vec<u8>>
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

impl StateMachine for MemKVStateMachine{

    fn get_last_index(&self)->u64{
        self.last_index
    }

    fn get_last_term(&self)->u64{
        self.last_term
    }

    fn query(&self,key :&[u8])->Option<Vec<u8>>{
        match self.state.get(key){
            Some(value) => Some(value.clone()),
            None => None,
        }
    }
    fn apply(&mut self,key :Vec<u8>,value :Vec<u8>)->bool{
        self.state.insert(key.to_vec(),value.to_vec()).is_some()
    }
    fn snapshot(&mut self)->Snapshot{
        Snapshot{
            last_index:self.last_index,
            last_term:self.last_term,
        }
    }
    fn restore_snapshot(&mut self,_snapshot :Snapshot)->std::io::Result<()>{
        Ok(())
    }
}

mod tests{
    #[test]
    fn test1() {
        use super::{StateMachine,MemKVStateMachine};
        let mut state_mechine = MemKVStateMachine::default();
        state_mechine.apply(b"hello".to_vec(), b"world".to_vec());
        let res = state_mechine.query(b"hello");
        assert_eq!(res.unwrap(),b"world");
    }
}


