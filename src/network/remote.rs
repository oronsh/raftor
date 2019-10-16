use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::oneshot;
use actix_raft::{
    AppData,
    messages,
};

use crate::network::Node;

pub trait RemoteMessage: Message + Send + Serialize + DeserializeOwned
where Self::Result: Send + Serialize + DeserializeOwned {
    fn type_id() -> &'static str;
}

pub struct RemoteMessageResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub rx: oneshot::Receiver<String>,
    pub m: PhantomData<M>,
}

impl<M> MessageResponse<Node, SendRaftMessage<M>> for RemoteMessageResult<M>
where
      M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<SendRaftMessage<M>>>(self, _: &mut Context<Node>, tx: Option<R>) {
        Arbiter::spawn(
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

pub trait RemoteMessageHandler: Send {
    fn handle(&self, msg: String, sender: oneshot::Sender<String>);
}

pub struct Provider<M>
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned
{
    pub recipient: Recipient<M>,
}

impl<M> RemoteMessageHandler for Provider<M>
where
    M: RemoteMessage + 'static,
    M::Result: Send + Serialize + DeserializeOwned
{
    fn handle(&self, msg: String, sender: oneshot::Sender<String>) {
        let msg = serde_json::from_slice::<M>(msg.as_ref()).unwrap();

        Arbiter::spawn(
            self.recipient.send(msg)
                .then(|res| {
                    match res {
                        Ok(res) => {
                            let body = serde_json::to_string::<M::Result>(&res).unwrap();
                            let _ = sender.send(body);
                        },
                        Err(_) => ()
                    }

                    Ok(())
                })
        );
    }
}
pub struct SendRaftMessage<M>(pub M)
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned;

impl<M> Message for SendRaftMessage<M>
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    type Result = M::Result;
}

#[derive(Message)]
pub struct RegisterMessage {
    pub type_id: &'static str,
    pub handler: Box<dyn RemoteMessageHandler>,
}

/// Impl RemoteMessage for RaftNetwork messages
impl<D: AppData> RemoteMessage for messages::AppendEntriesRequest<D> {
    fn type_id() -> &'static str { "AppendEntriesrequest" }
}

impl RemoteMessage for messages::VoteRequest {
    fn type_id() -> &'static str { "VoteRequest" }
}

impl RemoteMessage for messages::InstallSnapshotRequest {
    fn type_id() -> &'static str { "InstallSnapshotRequest" }
}
