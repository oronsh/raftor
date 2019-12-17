extern crate actix_raft;

use actix::prelude::*;
use actix_raft::{
    config::{Config, SnapshotPolicy},
    NodeId, Raft, RaftMetrics,
};

use crate::hash_ring::RingType;
use crate::network::Network;
use crate::server::{Server};
use std::time::Duration;
use tempfile::tempdir_in;

pub mod network;
pub mod storage;
mod client;

pub use self::{
    client::{RaftClient, InitRaft, AddNode, RemoveNode}
};

use self::storage::{MemoryStorage, MemoryStorageData, MemoryStorageError, MemoryStorageResponse};

pub type MemRaft =
    Raft<MemoryStorageData, MemoryStorageResponse, MemoryStorageError, Network, MemoryStorage>;

pub struct RaftBuilder;

impl RaftBuilder {
    pub fn new(
        id: NodeId,
        members: Vec<NodeId>,
        network: Addr<Network>,
        ring: RingType,
        server: Addr<Server>,
    ) -> Addr<MemRaft> {
        let id = id;
        let raft_members = members.clone();
        let metrics_rate = 1;
        let temp_dir = tempdir_in("/tmp").expect("Tempdir to be created without error.");
        let snapshot_dir = temp_dir.path().to_string_lossy().to_string();
        let config = Config::build(snapshot_dir.clone())
            .election_timeout_min(800)
            .election_timeout_max(5000)
            .heartbeat_interval(300)
            .metrics_rate(Duration::from_secs(metrics_rate))
            .snapshot_policy(SnapshotPolicy::default())
            .snapshot_max_chunk_size(10000)
            .validate()
            .expect("Raft config to be created without error.");

        let storage =
            MemoryStorage::create(move |_| MemoryStorage::new(raft_members, snapshot_dir, ring, server));

        let raft_network = network.clone();
        let raft_storage = storage.clone();

        Raft::create(move |_| {
            Raft::new(
                id,
                config,
                raft_network.clone(),
                raft_storage,
                raft_network.recipient(),
            )
        })
    }
}
