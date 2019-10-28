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
use crate::server;

pub trait RemoteMessage: Message + Send + Serialize + DeserializeOwned
where Self::Result: Send + Serialize + DeserializeOwned {
    fn type_id() -> &'static str;
    fn msg_type() -> MsgTypes;
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

impl<M> MessageResponse<Node, DistributeMessage<M>> for RemoteMessageResult<M>
where
      M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<DistributeMessage<M>>>(self, _: &mut Context<Node>, tx: Option<R>) {
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

/// DistributeMessage(Message)
pub struct DistributeMessage<M>(pub M)
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned;

impl<M> Message for DistributeMessage<M>
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    type Result = M::Result;
}

#[derive(Message, Clone)]
pub struct RegisterHandler(pub Addr<MemRaft>);

/// Impl RemoteMessage for RaftNetwork messages
impl<D: AppData> RemoteMessage for messages::AppendEntriesRequest<D> {
    fn type_id() -> &'static str { "AppendEntriesRequest" }
    fn msg_type() -> MsgTypes { MsgTypes::Raft }
}

impl RemoteMessage for messages::VoteRequest {
    fn type_id() -> &'static str { "VoteRequest" }
    fn msg_type() -> MsgTypes { MsgTypes::Raft }
}

impl RemoteMessage for messages::InstallSnapshotRequest {
    fn type_id() -> &'static str { "InstallSnapshotRequest" }
    fn msg_type() -> MsgTypes { MsgTypes::Raft }
}

impl<D: AppData, R: AppDataResponse, E: AppError> RemoteMessage for messages::ClientPayload<D, R, E> {
    fn type_id() -> &'static str { "ClientPayload" }
    fn msg_type() -> MsgTypes { MsgTypes::Raft }
}

/// Impl RemoteMessage for Application Messages
impl RemoteMessage for server::Join {
    fn type_id() -> &'static str { "Join" }
    fn msg_type() -> MsgTypes { MsgTypes::App }
}

impl RemoteMessage for server::SendRoom {
    fn type_id() -> &'static str { "SendRoom" }
    fn msg_type() -> MsgTypes { MsgTypes::App }
}

impl RemoteMessage for server::SendRecipient {
    fn type_id() -> &'static str { "SendRecipient" }
    fn msg_type() -> MsgTypes { MsgTypes::App }
}

impl RemoteMessage for server::CreateRoom {
    fn type_id() -> &'static str { "CreateRoom" }
    fn msg_type() -> MsgTypes { MsgTypes::App }
}

impl RemoteMessage for server::GetMembers {
    fn type_id() -> &'static str { "GetMembers" }
    fn msg_type() -> MsgTypes { MsgTypes::App }
}
