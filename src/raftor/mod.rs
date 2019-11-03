use actix::prelude::*;
use actix_raft::{NodeId};
use config;
use std::env;
use std::sync::{Arc, RwLock};

use crate::raft::{RaftBuilder, MemRaft};
use crate::network::{
    Network,
    HandlerRegistry,

};
use crate::hash_ring::{self, RingType};
use crate::config::{ConfigSchema};
use crate::utils;
use crate::server::{Server};

mod handlers;
mod raft;
use self::raft::{ClientRequest, InitRaft};

pub struct Raftor {
    id: NodeId,
    raft: Option<Addr<MemRaft>>,
    pub net: Addr<Network>,
    pub server: Addr<Server>,
    ring: RingType,
    registry: Arc<RwLock<HandlerRegistry>>,
}

impl Raftor {
    pub fn new() -> Raftor {
        let mut config = config::Config::default();

        config
            .merge(config::File::with_name("Config")).unwrap()
            .merge(config::Environment::with_prefix("APP")).unwrap();

        let config = config.try_into::<ConfigSchema>().unwrap();

        // create consistent hash ring
        let ring = hash_ring::Ring::new(10);

        // create handlers registry
        let registry = Arc::new(RwLock::new(HandlerRegistry::new()));

        // create application network
        let mut net = Network::new(ring.clone(), registry.clone());

        let args: Vec<String> = env::args().collect();
        let local_address = args[1].as_str();

        // generate local node id
        let node_id = utils::generate_node_id(local_address);

        // configure network
        net.configure(config);
        // listen on ip and port
        net.bind(local_address);
        // start network actor
        let net_addr = net.start();

        let server = Server::new(net_addr.clone(), ring.clone(), node_id);
        let server_addr = server.start();

        Raftor {
            id: node_id,
            net: net_addr,
            raft: None,
            server: server_addr,
            ring: ring,
            registry: registry
        }
    }
}

impl Actor for Raftor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify(InitRaft);
    }
}
