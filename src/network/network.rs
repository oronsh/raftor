use actix::prelude::*;
use actix_web::client::Client;
use actix_raft::{NodeId, RaftMetrics};
use log::debug;
use serde::{de::DeserializeOwned, Serialize, Deserialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::codec::FramedRead;
use tokio::io::AsyncRead;
use tokio::net::{TcpListener, TcpStream};
use tokio::timer::Delay;

use crate::network::{
    remote::{RemoteMessage, SendRemoteMessage, DispatchMessage},
    HandlerRegistry, Node, NodeCodec, NodeSession,
};

use crate::config::{ConfigSchema, NodeInfo, NetworkType};
use crate::hash_ring::RingType;
use crate::raft::{
    storage::{self, *},
    RaftClient,
    RemoveNode,
    AddNode,
};
use crate::server;
use crate::utils::generate_node_id;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum NetworkState {
    Initialized,
    SingleNode,
    Cluster,
}

pub struct Network {
    id: NodeId,
    net_type: NetworkType,
    address: Option<String>,
    discovery_host: String,
    peers: Vec<String>,
    nodes: BTreeMap<NodeId, Addr<Node>>,
    nodes_connected: Vec<NodeId>,
    pub isolated_nodes: Vec<NodeId>,
    nodes_info: HashMap<NodeId, NodeInfo>,
    server: Option<Addr<server::Server>>,
    state: NetworkState,
    metrics: Option<RaftMetrics>,
    sessions: BTreeMap<NodeId, Addr<NodeSession>>,
    ring: RingType,
    raft: Addr<RaftClient>,
    registry: Arc<RwLock<HandlerRegistry>>,
    info: NodeInfo,
    join_mode: bool,
}

impl Network {
    pub fn new(id: NodeId, ring: RingType, registry: Arc<RwLock<HandlerRegistry>>, net_type: NetworkType, raft: Addr<RaftClient>, discovery_host: String, info: NodeInfo) -> Network {
        Network {
            id: id,
            address: None,
            net_type: net_type,
            discovery_host: discovery_host,
            peers: Vec::new(),
            nodes: BTreeMap::new(),
            nodes_connected: Vec::new(),
            isolated_nodes: Vec::new(),
            nodes_info: HashMap::new(),
            server: None,
            state: NetworkState::Initialized,
            metrics: None,
            sessions: BTreeMap::new(),
            ring: ring,
            raft: raft,
            registry: registry,
            info: info,
            join_mode: false,
        }
    }

    pub fn configure(&mut self, config: ConfigSchema) {
        let nodes = config.nodes;

        for node in nodes.iter() {
            let id = generate_node_id(node.cluster_addr.as_str());
            self.nodes_info.insert(id, node.clone());
        }
    }

    /// register a new node to the network
    pub fn register_node(&mut self, id: NodeId, info: &NodeInfo, addr: Addr<Self>) {
        let info = info.clone();

        let network_address = self.address.as_ref().unwrap().clone();
        let local_id = self.id;
        let net_type = self.net_type.clone();
        let peer_addr = match self.net_type {
            NetworkType::App => info.app_addr.clone(),
            NetworkType::Cluster => info.cluster_addr.clone(),
        };

        if peer_addr == *network_address {
            return ();
        }

        self.restore_node(id); // restore node if needed

        if !self.nodes.contains_key(&id) {
            let node = Node::new(id, local_id, peer_addr, addr, net_type, self.info.clone()).start();
            self.nodes.insert(id, node);
        }
    }

    /// get a node from the network by its id
    pub fn get_node(&self, id: NodeId) -> Option<&Addr<Node>> {
        self.nodes.get(&id)
    }

    pub fn bind(&mut self, address: &str) {
        self.address = Some(address.to_owned());
    }

    /// Isolate the network of the specified node.
    pub fn isolate_node(&mut self, id: NodeId) {
        if let Some((idx, _)) = self.isolated_nodes.iter().enumerate().find(|(_, e)| *e == &id) {
            return ();
        }

        debug!("Isolating network for node {}.", &id);
        self.isolated_nodes.push(id);
    }

    /// Restore the network of the specified node.
    pub fn restore_node(&mut self, id: NodeId) {
        if let Some((idx, _)) = self.isolated_nodes.iter().enumerate().find(|(_, e)| *e == &id) {
            debug!("Restoring network for node {}.", &id);
            self.isolated_nodes.remove(idx);
        }
    }
}

#[derive(Message)]
pub struct NodeDisconnect(pub NodeId);

impl Handler<NodeDisconnect> for Network {
    type Result = ();

    fn handle(&mut self, msg: NodeDisconnect, ctx: &mut Context<Self>) {
        let id = msg.0;
        self.isolated_nodes.push(id);
        self.nodes_info.remove(&id);
        self.nodes.remove(&id);

        if self.net_type != NetworkType::Cluster {
            return ();
        }

        Arbiter::spawn(self.raft.send(RemoveNode(id))
            .map_err(|_| ())
            .and_then(|res| {
                futures::future::ok(())
            }))
    }
}

#[derive(Message)]
pub struct Handshake(pub NodeId, pub NodeInfo);

impl Handler<Handshake> for Network {
    type Result = ();

    fn handle(&mut self, msg: Handshake, ctx: &mut Context<Self>) {
        println!("Registering node {:?}", msg.1);
        self.nodes_info.insert(msg.0, msg.1.clone());
        self.register_node(msg.0, &msg.1, ctx.address().clone());
    }
}

#[derive(Message)]
pub struct RestoreNode(pub NodeId);

impl Handler<RestoreNode> for Network {
    type Result = ();

    fn handle(&mut self, msg: RestoreNode, ctx: &mut Context<Self>) {
        let id = msg.0;
        self.restore_node(id);
    }
}

pub struct GetClusterState;

impl Message for GetClusterState {
    type Result = Result<NetworkState, ()>;
}

impl Handler<GetClusterState> for Network {
    type Result = Result<NetworkState, ()>;

    fn handle(&mut self, _: GetClusterState, ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.state.clone())
    }
}

#[derive(Message)]
pub struct SetClusterState(pub NetworkState);

impl Handler<SetClusterState> for Network {
    type Result = ();

    fn handle(&mut self, msg: SetClusterState, ctx: &mut Context<Self>) {
        self.state = msg.0;
    }
}

pub struct GetNodes;

impl Message for GetNodes {
    type Result = Result<HashMap<NodeId, NodeInfo>, ()>;
}

impl Handler<GetNodes> for Network {
    type Result = Result<HashMap<NodeId, NodeInfo>, ()>;

    fn handle(&mut self, _: GetNodes, ctx: &mut Context<Self>) -> Self::Result {
        let nodes = self.nodes_info.clone();
        println!("Getting nodes {:#?}", nodes);
        Ok(nodes)
    }
}

pub struct DiscoverNodes;

impl Message for DiscoverNodes {
    type Result = Result<(Vec<NodeId>, bool), ()>;
}

impl Handler<DiscoverNodes> for Network {
    type Result = ResponseActFuture<Self, (Vec<NodeId>, bool), ()>;

    fn handle(&mut self, _: DiscoverNodes, _: &mut Context<Self>) -> Self::Result {
        Box::new(
            fut::wrap_future::<_, Self>(Delay::new(Instant::now() + Duration::from_secs(5)))
                .map_err(|_, _, _| ())
                .and_then(|_, act: &mut Network, _| fut::result(Ok((act.nodes_connected.clone(), act.join_mode)))),
        )
    }
}

impl Actor for Network {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let network_address = self.address.as_ref().unwrap().clone();
        let cluster_state_route = format!("http://{}/cluster/state", self.discovery_host.as_str());
        let cluster_nodes_route = format!("http://{}/cluster/nodes", self.discovery_host.as_str());

        println!("Listening on {}", network_address);
        println!("Local node id: {}", self.id);

        self.listen(ctx);
        self.nodes_connected.push(self.id);

        let mut client = Client::default();

        fut::wrap_future::<_, Self>(client.get(cluster_state_route).send())
            .map_err(|_, _, _| ())
            .and_then(move |res, _, _| {
                let mut res = res;
                fut::wrap_future::<_, Self>(res.body()).then(move |resp, _, _| {
                    if let Ok(body) = resp {
                        let state = serde_json::from_slice::<Result<NetworkState, ()>>(&body)
                            .unwrap().unwrap();

                        if state == NetworkState::Cluster {
                            // TODO:: Send register command to cluster
                            return fut::Either::A(fut::wrap_future::<_, Self>(client.get(cluster_nodes_route).send())
                                                  .map_err(|e, _, _| println!("HTTP Cluster Error {:?}", e))
                                                  .and_then(|res, act, _| {
                                                      let mut res = res;
                                                      fut::wrap_future::<_, Self>(res.body()).then(|resp, act, _| {
                                                          if let Ok(body) = resp {
                                                              let nodes = serde_json::from_slice::<Result<HashMap<NodeId, NodeInfo>, ()>>(&body)
                                                                  .unwrap().unwrap();

                                                              act.nodes_info = nodes;
                                                              act.join_mode = true;
                                                          }

                                                          fut::ok(())
                                                      })
                                                  })
                                                  .and_then(|_, _, _| fut::ok(()))
                            );
                        }
                    }

                    fut::Either::B(fut::ok(()))
                })
            })
            .map_err(|e, _, _| println!("HTTP Cluster Error {:?}", e))
            .and_then(move |_, act, ctx| {
                let nodes = act.nodes_info.clone();
                println!("about to register nodes {:#?}", nodes);
                for (id, info) in &nodes {
                    act.register_node(*id, info, ctx.address().clone());
                }

                fut::ok(())
            })
            .spawn(ctx);
    }
}

#[derive(Message)]
struct NodeConnect(TcpStream);

impl Network {
    fn listen(&mut self, ctx: &mut Context<Self>) {
        let server_addr = self.address.as_ref().unwrap().as_str().parse().unwrap();
        let listener = TcpListener::bind(&server_addr).unwrap();

        ctx.add_message_stream(listener.incoming().map_err(|_| ()).map(NodeConnect));
    }
}

impl Handler<NodeConnect> for Network {
    type Result = ();

    fn handle(&mut self, msg: NodeConnect, ctx: &mut Context<Self>) {
        let addr = ctx.address();
        let registry = self.registry.clone();
        let net_type = self.net_type.clone();

        NodeSession::create(move |ctx| {
            let (r, w) = msg.0.split();
            NodeSession::add_stream(FramedRead::new(r, NodeCodec), ctx);
            NodeSession::new(
                actix::io::FramedWrite::new(w, NodeCodec, ctx),
                addr,
                registry,
                net_type
            )
        });
    }
}

pub struct GetNodeAddr(pub String);

impl Message for GetNodeAddr {
    type Result = Result<Addr<Node>, ()>;
}

impl Handler<GetNodeAddr> for Network {
    type Result = ResponseActFuture<Self, Addr<Node>, ()>;

    fn handle(&mut self, msg: GetNodeAddr, ctx: &mut Context<Self>) -> Self::Result {
        let res = fut::wrap_future(ctx.address().send(GetNode(msg.0)))
            .map_err(|_, _: &mut Network, _| println!("GetNodeAddr Error"))
            .and_then(|res, act, _| {
                if let Ok(info) = res {
                    let id = info.0;
                    let node = act.nodes.get(&id).unwrap();

                    fut::result(Ok(node.clone()))
                } else {
                    fut::result(Err(()))
                }
            });

        Box::new(res)
    }
}

pub struct GetNodeById(pub NodeId);

impl Message for GetNodeById {
    type Result = Result<Addr<Node>, ()>;
}

impl Handler<GetNodeById> for Network {
    type Result = Result<Addr<Node>, ()>;

    fn handle(&mut self, msg: GetNodeById, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(ref node) = self.get_node(msg.0) {
            Ok((*node).clone())
        } else {
            Err(())
        }
    }
}

#[derive(Message)]
pub struct DistributeMessage<M>(pub String, pub M)
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned;

pub struct DistributeAndWait<M>(pub String, pub M)
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned;

impl<M> Message for DistributeAndWait<M>
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned,
{
    type Result = Result<M::Result, ()>;
}

impl<M> Handler<DistributeMessage<M>> for Network
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned,
{
    type Result = ();

    fn handle(&mut self, msg: DistributeMessage<M>, ctx: &mut Context<Self>) -> Self::Result {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.0.clone()).unwrap();

        if let Some(ref node) = self.get_node(*node_id) {
            node.do_send(DispatchMessage(msg.1))
        }
    }
}

impl<M> Handler<DistributeAndWait<M>> for Network
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned,
{
    type Result = Response<M::Result, ()>;

    fn handle(&mut self, msg: DistributeAndWait<M>, ctx: &mut Context<Self>) -> Self::Result {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.0.clone()).unwrap();

        if let Some(ref node) = self.get_node(*node_id) {
            let fut = node
                .send(SendRemoteMessage(msg.1))
                .map_err(|_| ())
                .and_then(|res| futures::future::ok(res));

            Response::fut(fut)
        } else {
            Response::fut(futures::future::err(()))
        }
    }
}

pub struct GetNode(pub String);

impl Message for GetNode {
    type Result = Result<(NodeId, String), ()>;
}

impl Handler<GetNode> for Network {
    type Result = Result<(NodeId, String), ()>;

    fn handle(&mut self, msg: GetNode, ctx: &mut Context<Self>) -> Self::Result {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.0).unwrap();

        let default = NodeInfo {
            public_addr: "".to_owned(),
            app_addr: "".to_owned(),
            cluster_addr: "".to_owned(),
        };

        let node = self.nodes_info.get(node_id).unwrap_or(&default);
        Ok((*node_id, node.public_addr.to_owned()))
    }
}

#[derive(Message)]
pub struct PeerConnected(pub NodeId);

impl Handler<PeerConnected> for Network {
    type Result = ();

    fn handle(&mut self, msg: PeerConnected, _ctx: &mut Context<Self>) {
        self.nodes_connected.push(msg.0);
    }
}

pub struct GetCurrentLeader;

impl Message for GetCurrentLeader {
    type Result = Result<NodeId, ()>;
}

impl Handler<GetCurrentLeader> for Network {
    type Result = ResponseActFuture<Self, NodeId, ()>;

    fn handle(&mut self, msg: GetCurrentLeader, ctx: &mut Context<Self>) -> Self::Result {
        if let Some(ref mut metrics) = self.metrics {
            if let Some(leader) = metrics.current_leader {
                Box::new(fut::result(Ok(leader)))
            } else {
                Box::new(
                    fut::wrap_future::<_, Self>(ctx.address().send(msg))
                        .map_err(|_, _, _| ())
                        .and_then(|res, _, _| fut::result(res))
                )
            }
        } else {
            Box::new(
                    fut::wrap_future::<_, Self>(ctx.address().send(msg))
                        .map_err(|_, _, _| ())
                        .and_then(|res, _, _| fut::result(res))
                )
        }
    }
}


//////////////////////////////////////////////////////////////////////////////
// RaftMetrics ///////////////////////////////////////////////////////////////

impl Handler<RaftMetrics> for Network {
    type Result = ();

    fn handle(&mut self, msg: RaftMetrics, _: &mut Context<Self>) -> Self::Result {
        println!("Metrics: node={} state={:?} leader={:?} term={} index={} applied={} cfg={{join={} members={:?} non_voters={:?} removing={:?}}}",
               msg.id, msg.state, msg.current_leader, msg.current_term, msg.last_log_index, msg.last_applied,
               msg.membership_config.is_in_joint_consensus, msg.membership_config.members,
               msg.membership_config.non_voters, msg.membership_config.removing,
        );
        self.metrics = Some(msg);
    }
}
