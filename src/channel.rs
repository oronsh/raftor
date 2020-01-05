use actix::prelude::*;
use crate::session::{self, Session};
use actix_raft::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::hash_ring::RingType;
use crate::network::{remote::SendRemoteMessage, DistributeMessage, DistributeAndWait, GetNodeAddr, Network};

pub struct Channel {
    sessions: HashMap<String, WeakAddr<Session>>,
    net: WeakAddr<Network>,
    ring: RingType,
    node_id: NodeId,
}

impl Actor for Channel {
    type Context = Context<Self>;
}

impl Channel {
    fn new(net: WeakAddr<Network>, ring: RingType, node_id: NodeId) -> Self {
        sessions: HashMap::new(),
        net: net,
        ring: ring,
        node_id: node_id,
    }
}
pp#[derive(Message)]
pub struct Join(pub String, pub WeakAddr<Session>);

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct Publish {
    pub sender_id: String,
    pub content: String,
}

pub struct GetMembers;

impl Message for GetMembers {
    type Result = Result<Vec<String>, ()>;
}

impl Handler<GetMembers> for Channel {
    type Result = Response<Vec<String>, ()>;

    fn handle(&mut self, msg: GetMembers, ctx: &mut Context<Self>) -> Self::Result {
        let mut members: Vec<String> = Vec::new();

        for uid in sessions.iter() {
            members.push(uid.to_string());
        }

        Response::reply(Ok(members))
    }
}
