use actix::prelude::*;
use actix_raft::{
    admin::InitWithConfig, messages, NodeId, RaftMetrics,
    Raft,
};
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::codec::FramedRead;
use tokio::io::{AsyncRead, WriteHalf};

use crate::network::{
    Node, NodeSession,
    remote::{RemoteMessage, SendRemoteMessage},
    NodeCodec,
    NodeRequest,
    NodeResponse,
    HandlerRegistry,
    RemoteMessageHandler,
    Provider,
};

use crate::raft::{storage::{self, *}, MemRaft};
use crate::utils::generate_node_id;
use crate::config::{ConfigSchema, NodeList, NodeInfo};
use crate::server;
use crate::hash_ring::RingType;


pub type Payload = messages::ClientPayload<storage::MemoryStorageData, storage::MemoryStorageResponse, storage::MemoryStorageError>;

pub enum NetworkState {
    Initialized,
    SingleNode,
    Cluster,
}

pub struct Network {
    id: NodeId,
    address: Option<String>,
    peers: Vec<String>,
    nodes: HashMap<NodeId, Addr<Node>>,
    nodes_connected: Vec<NodeId>,
    nodes_info: HashMap<NodeId, NodeInfo>,
    server: Option<Addr<server::Server>>,
    state: NetworkState,
    metrics: Option<RaftMetrics>,
    sessions: HashMap<NodeId, Addr<NodeSession>>,
    ring: RingType,
    raft: Option<Addr<MemRaft>>,
    registry: Arc<HandlerRegistry>,
}

impl Network {
    pub fn new(ring: RingType, registry: Arc<HandlerRegistry>) -> Network {
        Network {
            id: 0,
            address: None,
            peers: Vec::new(),
            nodes: HashMap::new(),
            nodes_connected: Vec::new(),
            nodes_info: HashMap::new(),
            server: None,
            state: NetworkState::Initialized,
            metrics: None,
            sessions: HashMap::new(),
            ring: ring,
            raft: None,
            registry: registry,
        }
    }

    pub fn configure(&mut self, config: ConfigSchema) {
        let nodes = config.nodes;

        for node in nodes.iter() {
            let id = generate_node_id(node.private_addr.as_str());
            self.nodes_info.insert(id, node.clone());
            // register peers
            self.peers.push(node.private_addr.clone());
        }
    }

    /// register a new node to the network
    pub fn register_node(&mut self, peer_addr: &str, addr: Addr<Self>) {
        let id = generate_node_id(peer_addr);
        let local_id = self.id;
        let peer_addr = peer_addr.to_owned();
        let node = Supervisor::start(move |_| {
            Node::new(id, local_id, peer_addr, addr)
        });

        self.nodes.insert(id, node);
    }

    /// get a node from the network by its id
    pub fn get_node(&self, id: NodeId) -> Option<&Addr<Node>> {
        self.nodes.get(&id)
    }

    pub fn bind(&mut self, address: &str) {
        self.address = Some(address.to_owned());
        self.id = generate_node_id(address);
    }
}

pub struct DiscoverNodes;

impl Message for DiscoverNodes {
    type Result = Result<Vec<NodeId>, ()>;
}

impl Handler<DiscoverNodes> for Network {
    type Result = ResponseActFuture<Self, Vec<NodeId>, ()>;

    fn handle(&mut self, msg: DiscoverNodes, _: &mut Context<Self>) -> Self::Result {
        Box::new(fut::wrap_future::<_, Self>(Delay::new(Instant::now() + Duration::from_secs(5)))
                 .map_err(|_, _, _| ())
                 .and_then(|_, act: &mut Network, _| {
                     fut::result(Ok(act.nodes_connected.clone()))
                 }))
    }
}

impl Actor for Network {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let network_address = self.address.as_ref().unwrap().clone();

        println!("Listening on {}", network_address);
        println!("Local node id: {}", self.id);

        self.listen(ctx);
        self.nodes_connected.push(self.id);

        let peers = self.peers.clone();
        for peer in peers {
            if peer != *network_address {
                self.register_node(peer.as_str(), ctx.address().clone());
            }
        }
        /*
        ctx.run_later(Duration::new(10, 0), |act, ctx| {
            let num_nodes = act.nodes_connected.len();

            if num_nodes > 1 {
                println!("Starting cluster with {} nodes", num_nodes);
                act.state = NetworkState::Cluster;
                let network_addr = ctx.address();
                let members = act.nodes_connected.clone();
                let id = act.id;
                let raft_node = RaftNode::new(id, members, network_addr, act.ring.clone());

                act.raft = Some(raft_node);

                debug!("{:?}", act.nodes_connected.clone());

                ctx.spawn(
                    actix::fut::wrap_future(ctx.address().send(InitRaft))
                        .map_err(|_, _: &mut Network, _| ())
                        .and_then(|_, _, _| {
                            println!("Raft node init!");
                            fut::wrap_future(Delay::new(
                                Instant::now() + Duration::from_secs(5),
                            )).map_err(|_, _, _| ())
                        })
                        .and_then(|_, act, ctx: &mut Context<Network>| {
                            ctx.address().do_send(ClientRequest(act.id));
                            fut::ok(())
                        })
                );
            } else {
                println!("Starting in single node mode");
                act.state = NetworkState::SingleNode;
            }
        });
         */
    }
}

#[derive(Message)]
struct NodeConnect(TcpStream);

impl Network {
    fn listen(&mut self, ctx: &mut Context<Self>) {
        let server_addr = self.address.as_ref().unwrap().as_str().parse().unwrap();
        let listener = TcpListener::bind(&server_addr).unwrap();

        ctx.add_message_stream(
            listener.incoming()
                .map_err(|_| ())
                .map(NodeConnect)
        );
    }
}

impl Handler<NodeConnect> for Network {
    type Result = ();

    fn handle(&mut self, msg: NodeConnect, ctx: &mut Context<Self>) {
        let (r, w) = msg.0.split();
        let addr = ctx.address();
        let registry = self.registry.clone();

        NodeSession::create(move |ctx| {
            NodeSession::add_stream(FramedRead::new(r, NodeCodec), ctx);
            NodeSession::new(actix::io::FramedWrite::new(w, NodeCodec, ctx), addr, registry)
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
                println!("id: {:?} nodes {:?}", id, act.nodes.keys());
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

pub struct DistributeMessage<M>(&'static str, M)
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned;

impl<M> Message for DistributeMessage<M>
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    type Result = Result<M::Result, ()>;
}

impl<M> Handler<DistributeMessage<M>> for Network
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    type Result = Response<M::Result, ()>;

    fn handle(&mut self, msg: DistributeMessage<M>, ctx: &mut Context<Self>) -> Self::Result {
        let ring = self.ring.read().unwrap();
        let node_id = ring.get_node(msg.0.to_owned()).unwrap();

        if let Some(ref node) = self.get_node(*node_id) {
            let fut = node.send(SendRemoteMessage(msg.1))
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
            private_addr: "".to_owned(),
        };

        let node = self.nodes_info.get(node_id).unwrap_or(&default);
        Ok((*node_id, node.public_addr.to_owned()))
    }
}

pub struct SendToServer(pub String);

impl Message for SendToServer {
    type Result = Result<String, ()>;
}

impl Handler<SendToServer> for Network {
    type Result = Response<String, ()>;

    fn handle(&mut self, msg: SendToServer, _ctx: &mut Context<Self>) -> Self::Result {
        let body = msg.0;

        let res = Response::reply(Ok("".to_owned()));
        /*
            if let Some(ref mut server) = self.server {

            match type_id {
                ServerTypes::Join => {
                    let msg = serde_json::from_slice::<server::Join>(body.as_ref()).unwrap();
                    let future = server.send(msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                ServerTypes::SendRecipient => {
                    let msg = serde_json::from_slice::<server::SendRecipient>(body.as_ref()).unwrap();
                    let future = server.send(msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                ServerTypes::SendRoom => {
                    let msg = serde_json::from_slice::<server::SendRoom>(body.as_ref()).unwrap();
                    let future = server.send(msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                ServerTypes::CreateRoom => {
                    let msg = serde_json::from_slice::<server::CreateRoom>(body.as_ref()).unwrap();
                    let future = server.send(msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                ServerTypes::GetMembers => {
                    let msg = serde_json::from_slice::<server::GetMembers>(body.as_ref()).unwrap();
                    let future = server.send(msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                _ => Response::reply(Ok("".to_owned())),
            }
        } else {
            Response::reply(Ok("".to_owned()))
        };

             */

        res
    }
}

pub struct SendToRaft(pub String, pub String);

impl Message for SendToRaft {
    type Result = Result<String, ()>;
}

impl Handler<SendToRaft> for Network {
    type Result = Response<String, ()>;

    fn handle(&mut self, msg: SendToRaft, _ctx: &mut Context<Self>) -> Self::Result {
        let type_id = msg.0;
        let body = msg.1;
        let res = Response::reply(Ok("".to_owned()));
        /*
        let res = if let Some(ref mut raft) = self.raft {
            match type_id {
                MsgTypes::AppendEntriesRequest => {
                    let raft_msg = serde_json::from_slice::<
                            messages::AppendEntriesRequest<storage::MemoryStorageData>,
                        >(body.as_ref())
                        .unwrap();
                    let future = raft.addr.send(raft_msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                }
                MsgTypes::VoteRequest => {
                    let raft_msg =
                        serde_json::from_slice::<messages::VoteRequest>(body.as_ref()).unwrap();

                    let future = raft.addr.send(raft_msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                }
                MsgTypes::InstallSnapshotRequest => {
                    let raft_msg =
                        serde_json::from_slice::<messages::InstallSnapshotRequest>(body.as_ref())
                        .unwrap();

                    let future = raft.addr.send(raft_msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                MsgTypes::ClientPayload => {
                    let raft_msg =
                        serde_json::from_slice::<messages::ClientPayload<storage::MemoryStorageData, storage::MemoryStorageResponse, storage::MemoryStorageError>>(body.as_ref())
                        .unwrap();

                    let future = raft.addr.send(raft_msg).map_err(|_| ()).and_then(|res| {
                        let res_payload = serde_json::to_string(&res).unwrap();
                        futures::future::ok(res_payload)
                    });
                    Response::fut(future)
                },
                _ => Response::reply(Ok("".to_owned())),
            }
        } else {
            Response::reply(Ok("".to_owned()))
        };
        */
        res
    }
}

#[derive(Message)]
pub struct PeerConnected(pub NodeId);

impl Handler<PeerConnected> for Network {
    type Result = ();

    fn handle(&mut self, msg: PeerConnected, _ctx: &mut Context<Self>) {
        // println!("Registering node {}", msg.0);
        self.nodes_connected.push(msg.0);
        // self.sessions.insert(msg.0, msg.1);
    }
}

#[derive(Message)]
pub struct InitRaft;
/*
impl Handler<InitRaft> for Network {
    type Result = ();

    fn handle(&mut self, msg: InitRaft, _ctx: &mut Context<Self>) {
        let raft = self.raft.as_ref().unwrap();
        let init_msg = InitWithConfig::new(self.nodes_connected.clone());
        raft.addr.send(init_msg);
    }
}
 */

pub struct GetCurrentLeader;

impl Message for GetCurrentLeader {
    type Result = Result<NodeId, ()>;
}

impl Handler<GetCurrentLeader> for Network {
    type Result = Result<NodeId, ()>;

    fn handle(&mut self, msg: GetCurrentLeader, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(ref mut metrics) = self.metrics {
            if let Some(leader) = metrics.current_leader {
                Ok(leader)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

#[derive(Message)]
pub struct SetServer(pub Addr<server::Server>);

impl Handler<SetServer> for Network {
    type Result = ();

    fn handle(&mut self, msg: SetServer, _: &mut Context<Self>) {
        self.server = Some(msg.0);
    }
}

#[derive(Message)]
pub struct SetRaft(pub Addr<MemRaft>);

impl Handler<SetRaft> for Network {
    type Result = ();

    fn handle(&mut self, msg: SetRaft, _: &mut Context<Self>) {
        self.raft = Some(msg.0);
    }
}

pub struct ClientRequest(NodeId);

impl Message for ClientRequest {
    type Result = ();
}

impl Handler<ClientRequest> for Network {
    type Result = ();

    fn handle(&mut self, msg: ClientRequest, ctx: &mut Context<Self>) {
        /*
        let entry = messages::EntryNormal{data: storage::MemoryStorageData(msg.0)};
        let payload = Payload::new(entry, messages::ResponseMode::Applied);

        let req = fut::wrap_future(ctx.address().send(GetCurrentLeader))
            .map_err(|err, _: &mut Network, _| ())
            .and_then(move |res, act, ctx| {
                let leader = res.unwrap();
                println!("Found leader: {}", leader);
                // if leader is current node send message here
                if act.id == leader {
                    let raft = act.raft.as_ref().unwrap();
                    fut::Either::A(fut::wrap_future::<_, Self>(raft.addr.send(payload))
                                   .map_err(|_, _, _| ())
                                   .and_then(move |res, act, ctx| {
                                       match res {
                                           Ok(_) => {
                                               fut::ok(())
                                           },
                                           Err(err) => match err {
                                               messages::ClientError::Internal => {
                                                   debug!("TEST: resending client request.");
                                                   ctx.notify(msg);
                                                   fut::ok(())
                                               }
                                               messages::ClientError::Application(err) => {
                                                   panic!("Unexpected application error from client request: {:?}", err);
                                               }
                                               messages::ClientError::ForwardToLeader{leader, ..} => {
                                                   debug!("TEST: received ForwardToLeader error. Updating leader and forwarding.");
                                                   ctx.notify(msg);
                                                   fut::ok(())
                                               }
                                           }
                                       }
                                   })
                    )

                } else {
                    // send to remote raft
                    let node = act.get_node(leader).unwrap();
                    fut::Either::B(fut::wrap_future::<_, Self>(node.send(SendRaftMessage(payload)))
                                   .map_err(|_, _, _| ())
                                   .and_then(move |res, act, ctx| {
                                       match res {
                                           Ok(_) => {
                                               fut::ok(())
                                           },
                                           Err(err) => match err {
                                               messages::ClientError::Internal => {
                                                   debug!("TEST: resending client request.");
                                                   ctx.notify(msg);
                                                   fut::ok(())
                                               }
                                               messages::ClientError::Application(err) => {
                                                   panic!("Unexpected application error from client request: {:?}", err)
                                               }
                                               messages::ClientError::ForwardToLeader{leader, ..} => {
                                                   debug!("TEST: received ForwardToLeader error. Updating leader and forwarding.");
                                                   ctx.notify(msg);
                                                   fut::ok(())
                                               }
                                           }
                                       }
                                   })
                    )
                }
            });

        ctx.spawn(req);
*/
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
        self.metrics = Some(msg);
    }
}
