use actix::prelude::*;
use actix_raft::NodeId;
use config;
use std::env;
use std::sync::{Arc, RwLock};

use crate::config::{ConfigSchema, NetworkType};
use crate::hash_ring::{self, RingType};
use crate::network::{HandlerRegistry, Network, DiscoverNodes, SetClusterState, NetworkState};
use crate::raft::{RaftClient, MemRaft, RaftBuilder, InitRaft};
use crate::server::Server;
use crate::utils;

mod handlers;

pub struct Raftor {
    id: NodeId,
    raft: Addr<RaftClient>,
    pub app_net: Addr<Network>,
    pub cluster_net: Addr<Network>,
    pub server: Addr<Server>,
    discovery_host: String,
    ring: RingType,
    registry: Arc<RwLock<HandlerRegistry>>,
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

        let cluster_arb = Arbiter::new();
        let app_arb = Arbiter::new();
        let raft_arb = Arbiter::new();

        let raft_client = RaftClient::new(node_id, ring.clone(), registry.clone());
        let raft = RaftClient::start_in_arbiter(&raft_arb, |_| raft_client);

        // create cluster network
        let mut cluster_net = Network::new(node_id, ring.clone(), registry.clone(), NetworkType::Cluster, raft.clone(), config.discovery_host.clone());
        // create application network
        let mut app_net = Network::new(node_id, ring.clone(), registry.clone(), NetworkType::App, raft.clone(), config.discovery_host.clone());

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
            raft: raft,
            server: server_addr,
            ring: ring,
            registry: registry,
            discovery_host: config.discovery_host.clone(),
        }
    }
}

impl Actor for Raftor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        fut::wrap_future::<_, Self>(self.cluster_net.send(DiscoverNodes))
            .map_err(|err, _, _| panic!(err))
            .and_then(|res, act, ctx| {
                let nodes = res.unwrap();
                fut::wrap_future::<_, Self>(act.raft.send(InitRaft{ nodes, net: act.cluster_net.clone(), server: act.server.clone() }))
                    .map_err(|err, _, _| panic!(err))
                    .and_then(|_, act, _| {
                        act.app_net.do_send(SetClusterState(NetworkState::Cluster));
                        act.cluster_net.do_send(SetClusterState(NetworkState::Cluster));
                        fut::ok(())
                    })
            })
            .spawn(ctx);

        self.register_handlers();
    }
}
