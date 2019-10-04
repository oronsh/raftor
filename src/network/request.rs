use std::hash::{Hash, Hasher};
use std::collections::{HashMap};
use std::time::{Duration, Instant};
use fasthash::{XXHasher};
use actix::Message;
use futures::{Future, Async, Poll};
use serde::{Serialize, de::DeserializeOwned};
use std::rc::Rc;
use std::cell::RefCell;

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

pub type ReqTable = Rc<RefCell<HashMap<String, Request>>>;

pub enum RequestState {
    Initialized,
    Pending,
    Completed,
    Canceled,
    Timedout,
}

pub struct Request {
    pub id: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub state: RequestState,
    pub req: Option<NodeRequest>,
    pub res: Option<NodeResponse>,
}

impl Request {
    fn new() -> Request {
        let time = format!("{}", Instant::now());
        let id = hash(&time);

        Request {
            id,
            start_time: Instant::now(),
            end_time: None,
            state: RequestState::Initialized,
            req: None,
            res: None,
        }
    }

    pub fn req_from(&mut self, request: NodeRequest) -> Request {
        let mut req = Request::new();
        req.req = Some(request);
        req
    }

    pub fn res_from(&mut self, response: NodeResponse) -> Request {
        self.res = Some(response);
        self
    }

    pub fn create_req_table() -> ReqTable {
        Rc::new(RefCell::new(HashMap::new()))
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
