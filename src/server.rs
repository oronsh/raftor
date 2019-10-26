use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use actix_raft::NodeId;

use crate::session::{self, Session};
use crate::hash_ring::RingType;
use crate::network::{
    Network,
    remote::{DistributeMessage},
    GetNodeAddr,
};

pub struct Server {
    rooms: HashMap<String, HashSet<String>>,
    sessions: HashMap<String, Addr<Session>>,
    net: Addr<Network>,
    ring: RingType,
    node_id: NodeId,
}

impl Server {
    pub fn new(addr: Addr<Network>, ring: RingType, node_id: NodeId) -> Self {
        Server {
            rooms: HashMap::new(),
            sessions: HashMap::new(),
            net: addr,
            ring: ring,
            node_id: node_id,
        }
    }
}

#[derive(Message)]
pub struct Connect(pub String, pub Addr<Session>);

#[derive(Message)]
pub struct Disconnect(pub String);

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct Join {
    pub room_id: String,
    pub uid: String,
}

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct SendRecipient {
    pub recipient_id: String,
    pub uid: String,
}

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct SendRoom {
    pub room_id: String,
    pub uid: String,
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Connect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) {
        self.sessions.insert(msg.0, msg.1);
    }
}

impl Handler<Disconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        self.sessions.remove(&msg.0);
    }
}

impl Handler<session::Message> for Server {
    type Result = ();

    fn handle(&mut self, msg: session::Message, ctx: &mut Context<Self>) {

    }
}

impl Handler<Join> for Server {
    type Result = ();

    fn handle(&mut self, msg: Join, ctx: &mut Context<Self>) {
        if let Some(ref mut room) = self.rooms.get_mut(&msg.room_id) {
            room.insert(msg.uid);
        } else {
            Arbiter::spawn(self.net.send(GetNodeAddr(msg.uid.clone()))
                           .then(|res| {
                               let node = res.unwrap().unwrap();
                               node.do_send(DistributeMessage(msg));

                               futures::future::ok(())
                           }));
        }
    }
}

impl Handler<SendRecipient> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendRecipient, ctx: &mut Context<Self>) {

    }
}

impl Handler<SendRoom> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendRoom, ctx: &mut Context<Self>) {

    }
}

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct CreateRoom {
    pub room_id: String,
    pub uid: String,
}

impl Handler<CreateRoom> for Server {
    type Result = ();

    fn handle(&mut self, msg :CreateRoom, ctx: &mut Context<Self>) {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.room_id.clone()).unwrap();

        if *node_id != self.node_id {
            Arbiter::spawn(self.net.send(GetNodeAddr(msg.uid.clone()))
                           .then(|res| {
                               let node = res.unwrap().unwrap();
                               node.do_send(DistributeMessage(msg));

                               futures::future::ok(())
                           }));
            return;
        }

        if let Some(ref mut room) = self.rooms.get_mut(&msg.room_id) {
            return;
        } else {
            let mut users = HashSet::new();
            users.insert(msg.uid);
            self.rooms.insert(msg.room_id, users);
        }
    }
}
