extern crate actix;

use std::time::Duration;
use std::net::SocketAddr;
use futures::future;
use tokio::net::TcpStream;
use tokio::io::{AsyncRead, WriteHalf};
use tokio::codec::FramedRead;
use actix::prelude::*;

use crate::network::{
    Network,
    ClientNodeCodec,
    NodeRequest,
    NodeResponse,
};

enum NodeState {
    Registered,
    Connected
}

pub struct Node {
    id: u64,
    state: NodeState,
    peer_addr: String,
    framed: Option<actix::io::FramedWrite<WriteHalf<TcpStream>, ClientNodeCodec>>,
}

impl Node {
    pub fn new(id: u64, peer_addr: String) -> Addr<Self> {
        Node::create(move |_ctx| {
            Node {
                id: id,
                state: NodeState::Registered,
                peer_addr: peer_addr,
                framed: None,
            }
        })
    }

    fn connect(&mut self, ctx: &mut Context<Self>) {
        let remote_addr = self.peer_addr.as_str().parse().unwrap();
        let conn = TcpStream::connect(&remote_addr)
            .map_err(|e| {
                println!("Error: {:?}", e);
            })
            .map(TcpConnect)
            .into_stream();

        ctx.add_message_stream(conn);
    }
}

impl Actor for Node {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(10, 0), |act, ctx| {
            act.connect(ctx);
        });

    }
}

#[derive(Message)]
struct TcpConnect(TcpStream);

#[derive(Message)]
struct ResponseStream;

impl Handler<TcpConnect> for Node {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, ctx: &mut Context<Self>) {
        println!("Connection established!");
        let (r, w) = msg.0.split();
        Node::add_stream(FramedRead::new(r, ClientNodeCodec), ctx);
        self.framed = Some(actix::io::FramedWrite::new(w, ClientNodeCodec, ctx));

    }
}

impl actix::io::WriteHandler<std::io::Error> for Node {}

impl StreamHandler<NodeResponse, std::io::Error> for Node {
    fn handle(&mut self, msg: NodeResponse, ctx: &mut Context<Self>) {
        match msg {
            NodeResponse::Ping => {
                self.framed.as_mut().unwrap().write(NodeRequest::Ping);
            },
            _ => ()
        }
    }
}
