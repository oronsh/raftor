use actix::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::session::{Session};

pub struct Server {
    rooms: HashMap<String, HashSet<String>>,
    sessions: HashMap<String, Addr<Session>>,
}

#[derive(Message)]
pub struct Connect(pub String, pub Addr<Session>);

#[derive(Message)]
pub struct Disconnect(pub String);

#[derive(Message)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub room: Option<String>,
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

impl Handler<Message> for Server {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Context<Self>) {

    }
}
