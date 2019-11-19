use actix::prelude::*;
use actix::dev::ToEnvelope;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use std::marker::PhantomData;

use crate::network::remote::RemoteMessage;

pub trait RemoteMessageHandler: Send + Sync {
    fn handle(&self, msg: String, sender: Sender<String>);
}

/// Remote message handler
pub struct Provider<M, A>
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned,
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
{
    pub recipient: Addr<A>,
    pub m: PhantomData<M>,
}

impl<M, A> RemoteMessageHandler for Provider<M, A>
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned,
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
{
    fn handle(&self, msg: String, sender: Sender<String>) {
        let msg = serde_json::from_slice::<M>(msg.as_ref()).unwrap();
        Arbiter::spawn(self.recipient.send(msg).then(|res| {
            match res {
                Ok(res) => {
                    let body = serde_json::to_string(&res).unwrap();
                    let _ = sender.send(body);
                }
                Err(e) => (),
            }
            Ok::<_, ()>(())
        }))
    }
}

pub type Handlers = HashMap<&'static str, Arc<dyn RemoteMessageHandler>>;

pub struct HandlerRegistry {
    handlers: Handlers,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        HandlerRegistry {
            handlers: HashMap::new(),
        }
    }

    pub fn register<M, A>(&mut self, r: Addr<A>)
    where
        M: RemoteMessage + 'static,
        M::Result: Send + Serialize + DeserializeOwned,
        A: Actor + Handler<M>,
        A::Context: ToEnvelope<A, M>,
    {
        self.handlers
            .insert(M::type_id(), Arc::new(Provider { recipient: r, m: PhantomData }));
    }

    pub fn get(&self, type_id: &str) -> Option<&Arc<dyn RemoteMessageHandler>> {
        self.handlers.get(type_id)
    }
}
