use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::oneshot;
use log::{error};
use actix_raft::{
    AppData,
    AppError,
    AppDataResponse,
    messages,
};

use crate::network::{Node, MsgTypes};
use crate::raft::MemRaft;

pub trait RemoteMessage: Message + Send + Serialize + DeserializeOwned
where Self::Result: Send + Serialize + DeserializeOwned {
    fn type_id() -> MsgTypes;
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
                .map_err(|e| error!("{:?}", e))
                .and_then(move |msg| {
                    // Raft node has not been initialized yet
                    if msg == "" {
                        return Err(());
                    }

                    let msg = serde_json::from_slice::<M::Result>(msg.as_ref()).unwrap();
                    if let Some(tx) = tx {
                        let _ = tx.send(msg);
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

#[derive(Message, Clone)]
pub struct RegisterHandler(pub Addr<MemRaft>);

/// Impl RemoteMessage for RaftNetwork messages
impl<D: AppData> RemoteMessage for messages::AppendEntriesRequest<D> {
    fn type_id() -> MsgTypes { MsgTypes::AppendEntriesRequest }
}

impl RemoteMessage for messages::VoteRequest {
    fn type_id() -> MsgTypes { MsgTypes::VoteRequest }
}

impl RemoteMessage for messages::InstallSnapshotRequest {
    fn type_id() -> MsgTypes { MsgTypes::InstallSnapshotRequest }
}

impl<D: AppData, R: AppDataResponse, E: AppError> RemoteMessage for messages::ClientPayload<D, R, E> {
    fn type_id() -> MsgTypes { MsgTypes::ClientPayload }
}
