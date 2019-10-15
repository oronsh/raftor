extern crate actix_raft;
use actix_raft::{Raft};

use crate::network::{Network};

mod network;
mod storage;

use self::storage::{MemoryStorageData, MemoryStorageError, MemoryStorage};

pub type MemRaft = Raft<MemoryStorageData, MemoryStorageError, Network, MemoryStorage>;
