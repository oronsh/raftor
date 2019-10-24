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
};

pub struct Listener {
    network: Addr<Network>,
}

impl Listener {
    pub fn new(address: &str, network_addr: Addr<Network>) -> Addr<Listener> {
        let server_addr = address.parse().unwrap();
        let listener = TcpListener::bind(&server_addr).unwrap();

        Listener::create(|ctx| {
            ctx.add_message_stream(listener.incoming().map_err(|_| ()).map(NodeConnect));

            Listener {
                network: network_addr,
            }
        })
    }
}

impl Actor for Listener {
    type Context = Context<Self>;
}

#[derive(Message)]
struct NodeConnect(TcpStream);

impl Handler<NodeConnect> for Listener {
    type Result = ();

    fn handle(&mut self, msg: NodeConnect, _: &mut Context<Self>) {
        let (r, w) = msg.0.split();
        let network = self.network.clone();

        NodeSession::create(move |ctx| {
            NodeSession::add_stream(FramedRead::new(r, NodeCodec), ctx);
            NodeSession::new(actix::io::FramedWrite::new(w, NodeCodec, ctx), network)
        });
    }
}


// NodeSession
pub struct NodeSession {
    hb: Instant,
    network: Addr<Network>,
    framed: actix::io::FramedWrite<WriteHalf<TcpStream>, NodeCodec>,
    id: Option<NodeId>,
}

impl NodeSession {
    fn new(
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
            NodeRequest::Message(mid, type_id, body) => {
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
            },
        }
    }
}
