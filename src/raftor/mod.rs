use actix::prelude::*;
use actix_raft::NodeId;
use config;
use std::env;
use std::sync::{Arc, RwLock};

use crate::config::{ConfigSchema, NetworkType};
use crate::hash_ring::{self, RingType};
use crate::network::{HandlerRegistry, Network, DiscoverNodes};
use crate::raft::{RaftClient, MemRaft, RaftBuilder};
use crate::server::Server;
use crate::utils;

mod handlers;
mod raft;
pub use self::raft::{ClientRequest, InitRaft, AddNode, RemoveNode};

pub struct Raftor {
    id: NodeId,
    raft: Option<Addr<MemRaft>>,
    pub app_net: Addr<Network>,
    pub cluster_net: Addr<Network>,
    pub server: Addr<Server>,
    ring: RingType,
    registry: Arc<RwLock<HandlerRegistry>>,
}

impl Default for Raftor {
    fn default() -> Self {
        Raftor::new()
    }
}

impl Raftor {
    pub fn new() -> Raftor {
        let mut config = config::Config::default();

        config
            .merge(config::File::with_name("Config"))
            .unwrap()
            .merge(config::Environment::with_prefix("APP"))
            .unwrap();

        let config = config.try_into::<ConfigSchema>().unwrap();

        // create consistent hash ring
        let ring = hash_ring::Ring::new(10);

        // create handlers registry
        let registry = Arc::new(RwLock::new(HandlerRegistry::new()));


        let args: Vec<String> = env::args().collect();
        let cluster_address = args[1].as_str();
        let app_address = args[2].as_str();

        // generate local node id
        let node_id = utils::generate_node_id(cluster_address);

        // create cluster network
        let mut cluster_net = Network::new(node_id, ring.clone(), registry.clone(), NetworkType::Cluster);
        // create application network
        let mut app_net = Network::new(node_id, ring.clone(), registry.clone(), NetworkType::App);

        let cluster_arb = Arbiter::new();
        let app_arb = Arbiter::new();

        // create RaftClient actor
        // let raft = RaftClient

        cluster_net.configure(config.clone()); // configure network
        cluster_net.bind(cluster_address); // listen on ip and port

        app_net.configure(config.clone()); // configure network
        app_net.bind(app_address); // listen on ip and port

        let cluster_net_addr = Network::start_in_arbiter(&cluster_arb, |_| cluster_net);
        let app_net_addr = Network::start_in_arbiter(&app_arb, |_| app_net);

        let server = Server::new(app_net_addr.clone(), ring.clone(), node_id);
        let server_addr = server.start();

        Raftor {
            id: node_id,
            app_net: app_net_addr,
            cluster_net: cluster_net_addr,
            raft: None,
            server: server_addr,
            ring: ring,
            registry: registry,
        }
    }
}

impl Actor for Raftor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        fut::wrap_future::<_, Self>(self.cluster_net.send(DiscoverNodes))
            .map_err(|err, _, _| panic!(err))
            .and_then(|nodes, act, ctx| {
                act.raft.send(InitRaft{ nodes, net: act.cluster_net.clone() })
                    .into_actor(self)
                    .map_err(|err, _, _| panic!(err))
                    .map(|_, _, _| fut::ok())
            })
            .spawn(ctx);

        ctx.notify(InitRaft);
        self.register_handlers();
    }
}
