use std::collections::HashMap;

#[derive(Debug,PartialEq)]
pub struct StateMachine{
    last_index:u64,
    last_term:u64,
    state :HashMap<String,Vec<u8>>
}

impl StateMachine{
    pub fn new()->Self{
        StateMachine{
            last_index:0,
            last_term:0,
            state:HashMap::new()
        }
    }

    pub fn get_last_index(&self)->u64{
        self.last_index
    }

    pub fn get_last_term(&self)->u64{
        self.last_term
    }
}