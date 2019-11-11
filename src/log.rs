use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,PartialEq,Clone)]
pub struct LogEntry{
    index :u64,
    pub term :u64,
    data :Vec<u8>,
}


#[derive(Serialize, Deserialize, Debug,PartialEq)]
pub struct RaftLog{
    offset : u64,
    log_entrys : Vec<LogEntry>,
}

impl RaftLog{
    pub fn new(offset :u64) -> Self{
        RaftLog{
            offset,
            log_entrys:Vec::new(),
        }
    }

    pub fn get_last_index(&self)->u64{
        match self.log_entrys.last(){
            Some(last)=>last.index,
            None=>0,
        }
    }

    pub fn get_log_entry(&self,index :u64) -> LogEntry{
        let index = index - self.offset - 1;
        self.log_entrys[index as usize].clone()
    }

    pub fn get_log_entrys(&self,start_index :u64,end_index :u64) -> Vec<LogEntry>{
        let start_index = start_index - self.offset - 1;
        let end_index = end_index - self.offset - 1;
        self.log_entrys[start_index as usize..end_index as usize].to_vec()
    }

    pub fn get_last_term(&self)->u64{
        match self.log_entrys.last(){
            Some(last)=>last.term,
            None=>0,
        }
    }

    fn append_log_entry(&mut self,entry :LogEntry){
        self.offset += 1;
        self.log_entrys.push(entry);
    }

    fn append_log_entrys(&mut self,entrys :&mut Vec<LogEntry>){
        self.offset += entrys.len() as u64;
        self.log_entrys.append(entrys);
    }

}