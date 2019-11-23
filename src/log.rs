use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug,PartialEq,Clone)]
pub struct LogEntry{
    pub index :u64,
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

    pub fn latest_log_index(&self)->u64{
        match self.log_entrys.last(){
            Some(last)=>last.index,
            None=>0,
        }
    }

    pub fn latest_log_term(&self)->u64{
        match self.log_entrys.last(){
            Some(last)=>last.term,
            None=>0,
        }
    }

    pub fn term(&self,index :u64) -> u64{
        let index = index - self.offset;
        match self.log_entrys.get(index as usize){
            Some(item) => item.term,
            None => 0,
        }
    }

    pub fn entry(&self,index :u64)->Option<&LogEntry>{
        let index = index-self.offset;
        self.log_entrys.get(index as usize)
    }

    pub fn entries(&self,start_index :u64,end_index :u64) -> Option<Vec<LogEntry>>{
        println!("start_index : {} , end_index :{}", start_index,end_index);
        let start_index = start_index - self.offset;
        let end_index = end_index - self.offset;
        if start_index >= end_index{
            return None;
        }
        Some(self.log_entrys[start_index as usize..=end_index as usize].to_vec())
    }

    pub fn append(&mut self,index :u64,entries :Vec<LogEntry>){
        let mut index = (index - self.offset +1) as usize;
        self.offset += entries.len() as u64;
        for log in entries.into_iter(){
            match self.log_entrys.get(index){
                Some(exists_log)=>{
                    if exists_log.term != log.term{
                        self.log_entrys.truncate(index);
                        self.log_entrys.push(log);
                    }
                },
                None => {
                    self.log_entrys.push(log);
                }
            }
            index += 1;
        }
    }

    fn compression(){
        
    }

}