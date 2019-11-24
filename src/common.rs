use serde::{Serialize, Deserialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug,PartialEq,Eq,Clone,Hash)]
pub enum Command{
    PUT(K,V),
    GET(K),
    DELETE(K),
}

#[derive(Serialize, Deserialize, Debug,PartialEq,Eq,Clone,Hash)]
pub struct K(Vec<u8>);
#[allow(unused)]
impl K {
    pub fn as_vec(self) -> Vec<u8> {
        self.0
    }
}
impl From<Vec<u8>> for K {
    fn from(val: Vec<u8>) -> K {
        K(val)
    }
}
impl Into<Vec<u8>> for K {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Serialize, Deserialize, Debug,PartialEq,Eq,Clone,Hash)]
pub struct V(Vec<u8>);
#[allow(unused)]
impl V{
    pub fn as_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for V {
    fn from(val: Vec<u8>) -> V {
        V(val)
    }
}
impl Into<Vec<u8>> for V {
    fn into(self) -> Vec<u8> {
        self.0
    }
}