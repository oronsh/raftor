use actix::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::oneshot::{Sender};

use crate::network::{
    remote::{
        RemoteMessage
    }
};

pub trait RemoteMessageHandler: Send {
    fn handle(&self, msg: String, sender: Sender<String>);
}

/// Remote message handler
pub struct Provider<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub recipient: Recipient<M>,
}

impl<M> RemoteMessageHandler for Provider<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn handle(&self, msg: String, sender: Sender<String>) {
        let msg = serde_json::from_slice::<M>(msg.as_ref()).unwrap();
        Arbiter::spawn(
            self.recipient.send(msg).then(|res| {
                match res {
                    Ok(res) => {
                        let body = serde_json::to_string(&res).unwrap();
                        let _ = sender.send(body);
                    },
                    Err(e) => (),
                }
                Ok::<_, ()>(())
            }))
    }
}
