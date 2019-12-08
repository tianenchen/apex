use std::{io,error,fmt};

use serde::{Serialize, Deserialize};

use futures::channel;

pub type Result<T> = std::result::Result<T,Error>;

#[derive(Debug)]
pub enum Error{
    Io(io::Error),
    AsyncIo(async_std::io::Error),
    Send(channel::mpsc::SendError),
    Serialize(bincode::Error),
    // FindNewerLeader,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Io(err) => err.description(),
            Error::AsyncIo(err) => err.description(),
            // Error::FindNewerLeader => "FindNewerLeader",
            Error::Send(err) => err.description(),
            Error::Serialize(err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Error::Io(err) => Some(err),
            Error::AsyncIo(err) => Some(err),
            // Error::FindNewerLeader => None,
            Error::Send(err) => Some(err),
            Error::Serialize(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::AsyncIo(err) => write!(f, "Async IO error: {}", err),
            // Error::FindNewerLeader => write!(f,"Raft error find newer leader"),
            Error::Send(err) => write!(f, "Send error: {}", err),
            Error::Serialize(err) => write!(f, "De/Serialize error: {}", err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<channel::mpsc::SendError> for Error {
    fn from(err: channel::mpsc::SendError) -> Error {
        Error::Send(err)
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        Error::Serialize(err)
    }
}

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