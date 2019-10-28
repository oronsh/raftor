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
    pub content: String,
}

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct SendRoom {
    pub room_id: String,
    pub uid: String,
    pub content: String,
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
        println!("members: {:?}", self.sessions.keys());
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

            Arbiter::spawn(self.net.send(GetNodeAddr(msg.room_id.clone()))
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
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.recipient_id.clone()).unwrap();

        if let Some(session) = self.sessions.get(&msg.recipient_id) {
            // user found on this server
            session.do_send(session::TextMessage{
                content: msg.content,
                sender_id: msg.uid,
            });
        } else if (*node_id != self.node_id) {
            Arbiter::spawn(self.net.send(GetNodeAddr(msg.recipient_id.clone()))
                           .then(|res| {
                               let node = res.unwrap().unwrap();
                               node.do_send(DistributeMessage(msg));

                               futures::future::ok(())
                           }));
            return;

        }
    }
}

impl Handler<SendRoom> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendRoom, ctx: &mut Context<Self>) {
        if let Some(ref mut sessions) = self.rooms.get_mut(&msg.room_id) {
            for uid in sessions.iter() {
                if *uid != msg.uid {
                    ctx.notify(SendRecipient{
                        recipient_id: uid.clone(),
                        uid: msg.uid.clone(),
                        content: msg.content.clone(),
                    });
                }
            }
        } else {
            let ring = self.ring.read().unwrap();
            let node_id = ring.get_node(msg.room_id.clone()).unwrap();
            if *node_id != self.node_id {
                Arbiter::spawn(self.net.send(GetNodeAddr(msg.room_id.clone()))
                               .then(|res| {
                                   let node = res.unwrap().unwrap();
                                   node.do_send(DistributeMessage(msg));

                                   futures::future::ok(())
                               }));
                println!("Distributing message to node {}", node_id);
            }
        }
    }
}

#[derive(Message, Serialize, Deserialize, Debug)]
pub struct CreateRoom {
    pub room_id: String,
}

impl Handler<CreateRoom> for Server {
    type Result = ();

    fn handle(&mut self, msg :CreateRoom, ctx: &mut Context<Self>) {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.room_id.clone()).unwrap();

        if *node_id != self.node_id {
            Arbiter::spawn(self.net.send(GetNodeAddr(msg.room_id.clone()))
                           .then(|res| {
                               let node = res.unwrap().unwrap();
                               node.do_send(DistributeMessage(msg));

                               futures::future::ok(())
                           }));
            println!("Distributing message to node {}", node_id);
            return;
        }

        if let Some(ref mut room) = self.rooms.get_mut(&msg.room_id) {
            return;
        } else {
            self.rooms.insert(msg.room_id.clone(), HashSet::new());
            println!("Room {} created", msg.room_id);
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetMembers {
    pub room_id: String,
}

impl Message for GetMembers {
    type Result = Result<Vec<String>, ()>;
}

impl Handler<GetMembers> for Server {
    type Result = Response<Vec<String>, ()>;

    fn handle(&mut self, msg: GetMembers, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(ref mut sessions) = self.rooms.get_mut(&msg.room_id) {
            let mut members: Vec<String> = Vec::new();

            for uid in sessions.iter() {
                members.push(uid.to_string());
            }

            Response::reply(Ok(members))
        } else {
            Response::fut(self.net.send(GetNodeAddr(msg.room_id.clone()))
                          .map_err(|_| ())
                          .then(|res| {
                              let node = res.unwrap().unwrap();
                              node.send(DistributeMessage(msg))
                                  .map_err(|_| ())
                                  .map(|res| res.unwrap_or(Vec::new()))
                          }))
        }
    }
}
