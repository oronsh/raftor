use actix::prelude::*;
use std::time::{Duration, Instant};
use tokio::codec::FramedRead;
use tokio::io::{AsyncRead, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use actix_raft::{NodeId};
use log::{error};

use crate::network::{
    Network,
    NodeCodec,
    NodeRequest,
    NodeResponse,
    PeerConnected,
    SendToRaft,
    MsgTypes,
    SendToServer,
};

use crate::server;

// NodeSession
pub struct NodeSession {
    hb: Instant,
    network: Addr<Network>,
    framed: actix::io::FramedWrite<WriteHalf<TcpStream>, NodeCodec>,
    id: Option<NodeId>,
}

impl NodeSession {
    pub fn new(
        framed: actix::io::FramedWrite<WriteHalf<TcpStream>, NodeCodec>,
        network: Addr<Network>,
    ) -> NodeSession {
        NodeSession {
            hb: Instant::now(),
            framed: framed,
            network,
            id: None,
        }
    }

    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
                println!("Client heartbeat failed, disconnecting!");
                ctx.stop();
            }

            // Reply heartbeat
            act.framed.write(NodeResponse::Ping);
        });
    }
}

impl Actor for NodeSession {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.hb(ctx);
    }
}

impl actix::io::WriteHandler<std::io::Error> for NodeSession {}

impl StreamHandler<NodeRequest, std::io::Error> for NodeSession {
    fn handle(&mut self, msg: NodeRequest, ctx: &mut Context<Self>) {
        match msg {
            NodeRequest::Ping => {
                self.hb = Instant::now();
                // println!("Server got ping from {}", self.id.unwrap());
            },
            NodeRequest::Join(id) => {
                self.id = Some(id);
                // self.network.do_send(PeerConnected(id));
            },
            NodeRequest::Message(mid, type_id, msg_type, body) => {
                match msg_type {
                    MsgTypes::App => {
                        let task = actix::fut::wrap_future(self.network.send(SendToServer(body)))
                            .map_err(|err, _: &mut NodeSession, _| {
                                error!("{:?}", err);
                            })
                            .and_then(move |res, act, _| {
                                let payload = res.unwrap();
                                act.framed.write(NodeResponse::Result(mid, payload));
                                actix::fut::result(Ok(()))
                            });
                        ctx.spawn(task);
                    },
                    _ => {
                        let task = actix::fut::wrap_future(self.network.send(SendToRaft(type_id, body)))
                            .map_err(|err, _: &mut NodeSession, _| {
                                error!("{:?}", err);
                            })
                            .and_then(move |res, act, _| {
                                let payload = res.unwrap();
                                act.framed.write(NodeResponse::Result(mid, payload));
                                actix::fut::result(Ok(()))
                            });
                        ctx.spawn(task);
                    }
                }
            },
        }
    }
}
