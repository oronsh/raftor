use std::time::Duration;
use std::net::SocketAddr;
use std::collections::HashMap;
use futures::future;
use tokio::net::TcpStream;
use tokio::io::{AsyncRead, WriteHalf};
use tokio::codec::FramedRead;
use actix::prelude::*;
use actix_raft::{NodeId};
use tokio::sync::oneshot;
use actix::dev::{MessageResponse, ResponseChannel};
use std::marker::PhantomData;

use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::network::{
    Network,
    ClientNodeCodec,
    NodeRequest,
    NodeResponse,
    PeerConnected,
};

#[derive(PartialEq)]
enum NodeState {
    Registered,
    Connected
}

pub struct Node {
    id: NodeId,
    mid: u64,
    state: NodeState,
    peer_addr: String,
    framed: Option<actix::io::FramedWrite<WriteHalf<TcpStream>, ClientNodeCodec>>,
    requests: HashMap<u64, oneshot::Sender<String>>,
}

impl Node {
    pub fn new(id: u64, peer_addr: String) -> Addr<Self> {
        Node::create(move |_ctx| {
            Node {
                id: id,
                mid: 0,
                state: NodeState::Registered,
                peer_addr: peer_addr,
                framed: None,
                requests: HashMap::new(),
            }
        })
    }

    fn connect(&mut self, ctx: &mut Context<Self>) {
        // node is already connected
        if self.state == NodeState::Connected {
            return ();
        }

        println!("Connecting to node #{}", self.id);

        let remote_addr = self.peer_addr.as_str().parse().unwrap();
        let conn = TcpStream::connect(&remote_addr)
            .map_err(|e| {
                println!("Error: {:?}", e);
            })
            .map(TcpConnect)
            .into_stream();

        ctx.add_message_stream(conn);
    }

    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.framed.as_mut().unwrap().write(NodeRequest::Ping);
            act.hb(ctx);
        });
    }
}

impl Actor for Node {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify(Connect);
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("Node #{} disconnected", self.id);
        // TODO: remove from network.nodes_connected
    }
}

#[derive(Message)]
struct TcpConnect(TcpStream);

#[derive(Message)]
struct Connect;

impl Handler<TcpConnect> for Node {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, ctx: &mut Context<Self>) {
        println!("Connected to remote node #{}", self.id);
        self.state = NodeState::Connected;
        let (r, w) = msg.0.split();
        Node::add_stream(FramedRead::new(r, ClientNodeCodec), ctx);
        self.framed = Some(actix::io::FramedWrite::new(w, ClientNodeCodec, ctx));

        self.framed.as_mut().unwrap().write(NodeRequest::Join(self.id));
        self.hb(ctx);
    }
}

struct SendRaftMessage<M>(pub M)
where M: Message + Send + Serialize + DeserializeOwned + 'static,
      M::Result: Send + Serialize + DeserializeOwned + 'static;

impl<M> Message for SendRaftMessage<M>
where M: Message + Send + Serialize + DeserializeOwned + 'static,
      M::Result: Send + Serialize + DeserializeOwned + 'static
{
    type Result = Result<M::Result, ()>;
}

impl<M> Handler<SendRaftMessage<M>> for Node
where M: Message + Send + Serialize + DeserializeOwned + 'static,
      M::Result: Send + Serialize + DeserializeOwned + 'static
{
    type Result = RemoteMessageResult<M>;

    fn handle(&mut self, msg: SendRaftMessage<M>, ctx: &mut Context<Self>) -> Self::Result {
        let (tx, rx) = oneshot::channel::<String>();

        if let Some(ref mut framed) = self.framed {
            self.mid += 1;
            self.requests.insert(self.mid, tx);

            let body = serde_json::to_string::<M>(&msg.0).unwrap();
            let request = NodeRequest::Message(0, body);
            framed.write(request);
        }

        Response::fut(futures::future::ok(NodeResponse::Ping))
    }
}

impl Handler<Connect> for Node {
    type Result = ();

    fn handle(&mut self, _msg: Connect, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(2, 0), |act, ctx| {
            act.connect(ctx);
            ctx.notify(Connect);
        });
    }
}

impl actix::io::WriteHandler<std::io::Error> for Node {}

impl StreamHandler<NodeResponse, std::io::Error> for Node {
    fn handle(&mut self, msg: NodeResponse, ctx: &mut Context<Self>) {
        match msg {
            NodeResponse::Result(mid, payload) => {
                if let Some(tx) = self.requests.remove(&mid) {
                    // let body = serde_json::from_slice::<>()
                }
            },
            NodeResponse::Ping => {
                println!("Client got Ping from {}", self.id);
              },
            _ => ()
        }
    }
}

struct RemoteMessageResult<M>
    where M: Message + Send + Serialize + DeserializeOwned + 'static,
          M::Result: Send + Serialize + DeserializeOwned + 'static
{
    rx: oneshot::Receiver<M::Result>,
    m: PhantomData<M>,
}

impl<M> MessageResponse<Node, SendRaftMessage<M>> for RemoteMessageResult<M>
where
      M: Message + Send + Serialize + DeserializeOwned + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<SendRaftMessage<M>>>(self, _: &mut Context<Node>, tx: Option<R>) {
        Arbiter::handle().spawn(
            self.rx
                .map_err(|e| ())
                .and_then(move |msg| {
                    let msg = serde_json::from_slice::<M::Result>(msg.as_ref()).unwrap();
                    if let Some(tx) = tx {
                        let _ = tx.send(msg);
                    }
                    Ok(())
                })
        );
    }
}
