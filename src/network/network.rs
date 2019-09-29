use crate::network::{Listener, Node};
use actix::prelude::*;
use std::collections::HashMap;
use crate::utils::{generate_node_id};

pub struct Network {
    id: u64,
    address: Option<String>,
    nodes: HashMap<u64, Addr<Node>>,
    nodes_connected: u64,
    listener: Option<Addr<Listener>>,
}

impl Network {
    pub fn new() -> Network {
        Network {
            id: 0,
            address: None,
            nodes: HashMap::new(),
            listener: None,
            nodes_connected: 0,
        }
    }

    // register a new node to the network
    pub fn register_node(&mut self, peer_addr: &str) {
        let id = generate_node_id(peer_addr);
        let node = Node::new(id, peer_addr.to_owned());
        self.nodes.insert(id, node);
    }

    // get a node from the network by its id
    pub fn get_node(&mut self, id: u64) -> Option<&Addr<Node>> {
        self.nodes.get(&id)
    }

    pub fn listen(&mut self, address: &str) {
        self.address = Some(address.to_owned());
        self.id = generate_node_id(address);
    }

    pub fn discover_peers(&mut self) {
        for node in self.nodes.iter() {

        }
    }
}

impl Actor for Network {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let network_address = self.address.as_ref().unwrap();

        println!("Listening on {}", network_address);
        println!("Local node id: {}", self.id);
        let listener_addr = Listener::new(network_address.as_str(), ctx.address().clone());
        self.listener = Some(listener_addr);
    }
}

#[derive(Message, Debug)]
pub struct PeerConnected(pub String);

impl Handler<PeerConnected> for Network {
    type Result = ();

    fn handle(&mut self, msg: PeerConnected, ctx: &mut Context<Self>) {
        println!("Registering peer addr {}", msg.0);
    }
}
