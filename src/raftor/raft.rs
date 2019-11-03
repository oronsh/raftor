use actix::prelude::*;
use actix_raft::{
    admin::InitWithConfig,
    messages::{
        ClientError,
        ClientPayload,
        ClientPayloadResponse,
        EntryNormal,
        ResponseMode,
    },
    NodeId,
    RaftMetrics,
    Raft,
};
use log::debug;

use crate::network::{
    GetCurrentLeader,
    GetNodeById,
    remote::{SendRemoteMessage},
};
use crate::raft::{
    storage::{
        MemoryStorageData,
        MemoryStorageResponse,
        MemoryStorageError,
    }
};
use crate::raftor::{Raftor};

pub type Payload = ClientPayload<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>;

pub struct ClientRequest(pub NodeId);

impl Message for ClientRequest {
    type Result = ();
}

impl Handler<ClientRequest> for Raftor {
    type Result = ();

    fn handle(&mut self, msg: ClientRequest, ctx: &mut Context<Self>) {
        let entry = EntryNormal{data: MemoryStorageData(msg.0)};
        let payload = Payload::new(entry, ResponseMode::Applied);

        ctx.spawn(
            fut::wrap_future::<_, Self>(self.net.send(GetCurrentLeader))
                .map_err(|err, _, _| panic!(err))
                .and_then(move |res, act, ctx| {
                    let leader = res.unwrap();
                    println!("Found leader: {}", leader);

                    if leader == act.id {
                        if let Some(ref raft) = act.raft {
                            fut::wrap_future::<_, Self>(raft.send(payload))
                                .map_err(|err, _, _| panic!(err))
                                .and_then(|res, act, ctx| fut::ok(handle_client_response(res, ctx, msg)));
                            return fut::ok(());
                        }
                    }

                    fut::wrap_future::<_, Self>(act.net.send(GetNodeById(leader)))
                        .map_err(|err, _: &mut Self, ctx| {
                            panic!("Node {} not found", leader);
                        })
                        .and_then(|node, act, ctx| {
                            fut::wrap_future::<_, Self>(node.unwrap().send(SendRemoteMessage(payload)))
                                .map_err(|err, _, _| panic!(err))
                                .and_then(|res, act, ctx| fut::ok(handle_client_response(res, ctx, msg)))
                        });

                    fut::ok(())
                })
        );
    }
}

type ClientResponseHandler = Result<ClientPayloadResponse<MemoryStorageResponse>, ClientError<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>>;

fn handle_client_response(
    res: ClientResponseHandler,
    ctx: &mut Context<Raftor>,
    msg: ClientRequest
) {
    match res {
        Ok(_) => (),
        Err(err) => match err {
            ClientError::Internal => {
                debug!("TEST: resending client request.");
                ctx.notify(msg);
            }
            ClientError::Application(err) => {
                panic!("Unexpected application error from client request: {:?}", err);
            }
            ClientError::ForwardToLeader{leader, ..} => {
                debug!("TEST: received ForwardToLeader error. Updating leader and forwarding.");
                ctx.notify(msg);
            }
        }
    }
}
