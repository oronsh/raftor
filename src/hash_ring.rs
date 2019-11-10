use hash_ring::HashRing;
use std::sync::{Arc, RwLock};

use actix_raft::NodeId;

pub type RingType = Arc<RwLock<HashRing<NodeId>>>;
pub struct Ring;

impl Ring {
    pub fn new(replicas: isize) -> RingType {
        Arc::new(RwLock::new(HashRing::new(Vec::new(), replicas)))
    }
}
