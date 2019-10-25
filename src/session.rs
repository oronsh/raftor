use actix::prelude::*;
use actix_web_actors::ws;
use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};
use serde_json;

use crate::server::{self, Server};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Session {
    id: String,
    uid: Option<String>,
    room: String,
    server: Addr<Server>,
    hb: Instant,
}

impl Session {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // let uid = act.id.to_owned()
                // notify chat server
                act.server.do_send(server::Disconnect(act.id.to_owned()));

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping("");
        });
    }
}

impl Actor for Session {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();

        self.server
            .send(server::Connect(self.id.to_owned(), addr.clone()))
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(_) => (),
                    _ => ctx.stop()
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify server
        self.server.do_send(server::Disconnect(self.id.to_owned()));
        Running::Stop
    }
}

#[derive(Message)]
pub struct Message(pub String);

impl Handler<Message> for Session {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

#[derive(Serialize, Deserialize)]
pub struct TextMessage {
    recipient: String,
    content: String,
    room: Option<String>,
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Session {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            },
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            },
            ws::Message::Text(msg) => {
                let msg = serde_json::from_slice::<TextMessage>(msg.as_ref()).unwrap();
                self.server.do_send(server::Message {
                    id: msg.recipient,
                    content: msg.content,
                    room: msg.room,
                });
            },
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
