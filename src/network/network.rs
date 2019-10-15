use actix_raft::{NodeId, RaftMetrics};
use std::time::Duration;
use actix::prelude::*;
use std::collections::{HashMap, BTreeMap};
use log::{debug};

use crate::network::{Listener, Node};
use crate::utils::generate_node_id;
use crate::raft::{MemRaft, RaftNode};


pub enum NetworkState {
    Initialized,
    SingleNode,
    Cluster,
}

pub struct Network {
    id: NodeId,
    address: Option<String>,
    raft: Option<RaftNode>,
    peers: Vec<String>,
    nodes: HashMap<NodeId, Addr<Node>>,
    nodes_connected: Vec<NodeId>,
    listener: Option<Addr<Listener>>,
    state: NetworkState,
    pub metrics: BTreeMap<NodeId, RaftMetrics>,
}

impl Network {
    pub fn new() -> Network {
        Network {
            id: 0,
            address: None,
            peers: Vec::new(),
            nodes: HashMap::new(),
            raft: None,
            listener: None,
            nodes_connected: Vec::new(),
            state: NetworkState::Initialized,
            metrics: BTreeMap::new(),
        }
    }

    // set peers
    pub fn peers(&mut self, peers: Vec<&str>) {
        for peer in peers.iter() {
            self.peers.push(peer.to_string());
        }
    }

    // register a new node to the network
    pub fn register_node(&mut self, peer_addr: &str, network: Addr<Network>) {
        let id = generate_node_id(peer_addr);
        let node = Node::new(id, peer_addr.to_owned());
        self.nodes.insert(id, node);
    }

    // get a node from the network by its id
    pub fn get_node(&mut self, id: NodeId) -> Option<&Addr<Node>> {
        self.nodes.get(&id)
    }

    pub fn listen(&mut self, address: &str) {
        self.address = Some(address.to_owned());
        self.id = generate_node_id(address);
    }

    pub fn discover_peers(&mut self) {
        for node in self.nodes.iter() {}
    }
}

impl Actor for Network {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let network_address = self.address.as_ref().unwrap().clone();

        println!("Listening on {}", network_address);
        println!("Local node id: {}", self.id);
        let listener_addr = Listener::new(network_address.as_str(), ctx.address().clone());
        self.listener = Some(listener_addr);
        self.nodes_connected.push(self.id); // push local id
        // register nodes
        let peers = self.peers.clone();
        for peer in peers {
            if peer != *network_address {
                self.register_node(peer.as_str(), ctx.address());
            }
        }

        ctx.run_later(Duration::new(20, 0), |act, ctx| {
            let num_nodes = act.nodes_connected.len();

            if num_nodes > 1 {
                println!("Starting cluster with {} nodes", num_nodes);
                act.state = NetworkState::Cluster;
                let network_addr = ctx.address();
                let members = act.nodes_connected.clone();
                let id = act.id;
                let raft_addr = RaftNode::new(id , members, network_addr);

                act.raft = Some(raft_addr);
            } else {
                println!("Starting in single node mode");
                act.state = NetworkState::SingleNode;
            }
        });
    }
}

#[derive(Message, Debug)]
pub struct PeerConnected(pub NodeId);

impl Handler<PeerConnected> for Network {
    type Result = ();

    fn handle(&mut self, msg: PeerConnected, ctx: &mut Context<Self>) {
        // println!("Registering node {}", msg.0);
        self.nodes_connected.push(msg.0);
    }
}

//////////////////////////////////////////////////////////////////////////////
// RaftMetrics ///////////////////////////////////////////////////////////////

impl Handler<RaftMetrics> for Network {
    type Result = ();

    fn handle(&mut self, msg: RaftMetrics, _: &mut Context<Self>) -> Self::Result {
        debug!("Metrics: node={} state={:?} leader={:?} term={} index={} applied={} cfg={{join={} members={:?} non_voters={:?} removing={:?}}}",
            msg.id, msg.state, msg.current_leader, msg.current_term, msg.last_log_index, msg.last_applied,
            msg.membership_config.is_in_joint_consensus, msg.membership_config.members,
            msg.membership_config.non_voters, msg.membership_config.removing,
        );
        self.metrics.insert(msg.id, msg);
    }
}
