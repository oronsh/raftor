use std::hash::{Hash, Hasher};
use std::collections::{HashMap};
use std::time::{Duration, Instant};
use fasthash::{XXHasher};
use actix::Message;
use futures::{Future, Async, Poll};
use serde::{Serialize, de::DeserializeOwned};
use actix_raft::{
    NodeId,
    messages::{
        AppendEntriesRequest,
        AppendEntriesResponse,
        InstallSnapshotRequest,
        InstallSnapshotResponse,
        VoteRequest,
        VoteResponse,
    }
};

use crate::utils::generate_node_id;
use crate::network::{NodeRequest, NodeResponse};
use crate::data::{Data};

pub enum RequestState {
    Initialized,
    Pending,
    Completed,
    Canceled,
    Timedout,
}

pub struct Request {
    id: String,
    start_time: Instant,
    end_time: Option<Instant>,
    state: RequestState,
    req: Option<NodeRequest>,
    res: Option<NodeResponse>,
}

impl<M: DeserializeOwned + Serialize + Message + 'static> Request {
    pub fn new() -> Request {
        let time = format!("{}", Instant::now());
        let h = hash(&time);
        let id = generate_node_id(h);

        Request {
            id,
            start_time: Instant::now(),
            end_time: None,
            state: RequestState::Initialized,
            req: None,
            res: None,
        }
    }

    pub fn req_from<M>(&mut self, request: M) -> Request {
        let req = match request {

        };
    }
}


impl Future for Request {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {

    }
}

fn hash<H: Hash>(t: &H) -> String {
    let mut s: XXHasher = Default::default();
    t.hash(&mut s);
    s.finish()
}
