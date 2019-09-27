use std::net::SocketAddr;
use futures::future;
use tokio::net::TcpStream;
use actix::prelude::*;

#[derive(Debug, PartialEq)]
enum NodeState {
    Registered,
    Connected
}

#[derive(Debug, PartialEq)]
pub struct Node {
    id: u64,
    state: NodeState,
    peer_addr: String
}

impl Node {
    pub fn new(id: u64, peer_addr: &str) -> Self {
        Node {
            id,
            state: NodeState::Registered,
            peer_addr: peer_addr.to_owned()
        }
    }
}
