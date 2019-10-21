extern crate actix_raft;

use actix::prelude::*;
use actix_raft::{
    NodeId,
    Raft,
    config::{Config, SnapshotPolicy},
};

use tempfile::{tempdir_in};
use std::time::{Duration};
use crate::network::{Network};

pub mod network;
pub mod storage;

use self::storage::{MemoryStorageData, MemoryStorageError, MemoryStorage, MemoryStorageResponse};

pub type MemRaft = Raft<MemoryStorageData, MemoryStorageResponse, MemoryStorageError, Network, MemoryStorage>;

pub struct RaftNode {
    id: NodeId,
    pub addr: Addr<MemRaft>,
    members: Vec<NodeId>,
    network: Addr<Network>,
}

impl RaftNode {
    pub fn new(id: NodeId, members: Vec<NodeId>, network: Addr<Network>) -> RaftNode {
        let id = id;
        let raft_members = members.clone();
        let metrics_rate = 1;
        let temp_dir = tempdir_in("/tmp").expect("Tempdir to be created without error.");
        let snapshot_dir = temp_dir.path().to_string_lossy().to_string();
        let config = Config::build(snapshot_dir.clone())
            .election_timeout_min(800).election_timeout_max(1000).heartbeat_interval(300)
            .metrics_rate(Duration::from_secs(metrics_rate))
            .snapshot_policy(SnapshotPolicy::Disabled).snapshot_max_chunk_size(10000)
            .validate().expect("Raft config to be created without error.");

        let storage = MemoryStorage::create(|_| MemoryStorage::new(raft_members, snapshot_dir));

        let raft_network = network.clone();
        let addr = Raft::create(move |_| {
            Raft::new(id, config, raft_network.clone(), storage.clone(), raft_network.recipient())
        });

        RaftNode {
            id: id,
            addr: addr,
            members: members,
            network: network,
        }
    }
}
