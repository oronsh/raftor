use actix::prelude::*;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::io::{WriteHalf};
use tokio::net::{TcpStream};
use actix_raft::{NodeId};
use std::sync::{Arc, RwLock};
use log::{error};

use crate::network::{
    Network,
    NodeCodec,
    NodeRequest,
    NodeResponse,
    HandlerRegistry,
};

// NodeSession
pub struct NodeSession {
    hb: Instant,
    network: Addr<Network>,
    framed: actix::io::FramedWrite<WriteHalf<TcpStream>, NodeCodec>,
    id: Option<NodeId>,
    registry: Arc<RwLock<HandlerRegistry>>,
}

impl NodeSession {
    pub fn new(
        framed: actix::io::FramedWrite<WriteHalf<TcpStream>, NodeCodec>,
        network: Addr<Network>,
        registry: Arc<RwLock<HandlerRegistry>>,
    ) -> NodeSession {
        NodeSession {
            hb: Instant::now(),
            framed: framed,
            network,
            id: None,
            registry: registry,
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
            },
            NodeRequest::Join(id) => {
                self.id = Some(id);
            },
            NodeRequest::Message(mid, type_id, body) => {
                let (tx, rx) = oneshot::channel();
                let registry = self.registry.read().unwrap();

                if let Some(ref handler) = registry.get(type_id.as_str()) {
                    handler.handle(body, tx);

                    fut::wrap_future::<_, Self>(rx)
                        .then(move |res, act, _| {
                            // println!("Got remote message {:?}", res);
                            match res {
                                Ok(res) => act.framed.write(NodeResponse::Result(mid, res)),
                                Err(_) => (),
                            }
                            fut::ok(())
                        }).spawn(ctx)
                }
            },
        }
    }
}
