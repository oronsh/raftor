use actix::prelude::*;
use actix_raft::{
    admin::InitWithConfig,
    messages::{ClientError, ClientPayload, ClientPayloadResponse, EntryNormal, ResponseMode},
    NodeId, Raft, RaftMetrics,
};
use log::debug;
use std::time::{Duration, Instant};
use tokio::timer::Delay;

use crate::network::{remote::SendRemoteMessage, DiscoverNodes, GetCurrentLeader, GetNodeById, SetRaft};
use crate::raft::{
    storage::{MemoryStorageData, MemoryStorageError, MemoryStorageResponse},
    RaftBuilder,
};
use crate::raftor::Raftor;

type ClientResponseHandler = Result<
    ClientPayloadResponse<MemoryStorageResponse>,
    ClientError<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>,
>;

pub type Payload = ClientPayload<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>;

#[derive(Message)]
pub struct InitRaft;

impl Handler<InitRaft> for Raftor {
    type Result = ();

    fn handle(&mut self, _: InitRaft, ctx: &mut Context<Self>) {
        ctx.spawn(
            fut::wrap_future::<_, Self>(self.cluster_net.send(DiscoverNodes))
                .map_err(|err, _, _| panic!(err))
                .and_then(|nodes, act, ctx| {
                    let nodes = nodes.unwrap_or(Vec::new());
                    let num_nodes = nodes.len();

                    let raft =
                        RaftBuilder::new(act.id, nodes.clone(), act.cluster_net.clone(), act.ring.clone());
                    act.raft = Some(raft);
                    act.register_handlers();

                    fut::wrap_future::<_, Self>(Delay::new(
                        Instant::now() + Duration::from_secs(5),
                    ))
                        .map_err(|_, _, _| ())
                        .and_then(move |_, act, ctx| {
                            fut::wrap_future::<_, Self>(
                                act.raft
                                    .as_ref()
                                    .unwrap()
                                    .send(InitWithConfig::new(nodes.clone())),
                            )
                                .map_err(|err, _, _| panic!(err))
                                .and_then(|_, _, _| {
                                    println!("Inited with config!");
                                    fut::wrap_future::<_, Self>(Delay::new(
                                        Instant::now() + Duration::from_secs(5),
                                    ))
                                })
                                .map_err(|_, _, _| ())
                                .and_then(|_, act, ctx| {
                                    ctx.notify(ClientRequest(act.id));
                                    fut::ok(())
                                })
                        })

                }),
        );
    }
}

pub struct ClientRequest(pub NodeId);

impl Message for ClientRequest {
    type Result = ();
}

impl Handler<ClientRequest> for Raftor {
    type Result = ();

    fn handle(&mut self, msg: ClientRequest, ctx: &mut Context<Self>) {
        let entry = EntryNormal {
            data: MemoryStorageData(msg.0),
        };
        let payload = Payload::new(entry, ResponseMode::Applied);

        ctx.spawn(
            fut::wrap_future::<_, Self>(self.cluster_net.send(GetCurrentLeader))
                .map_err(|err, _, _| panic!(err))
                .and_then(move |res, act, ctx| {
                    let leader = res.unwrap();

                    if leader == act.id {
                        if let Some(ref raft) = act.raft {
                            return fut::Either::A(
                                fut::wrap_future::<_, Self>(raft.send(payload))
                                    .map_err(|err, _, _| panic!(err))
                                    .and_then(|res, act, ctx| {
                                        fut::ok(handle_client_response(res, ctx, msg))
                                    }),
                            );
                        }
                    }

                    fut::Either::B(
                        fut::wrap_future::<_, Self>(act.cluster_net.send(GetNodeById(leader)))
                            .map_err(move |err, _, _| panic!("Node {} not found", leader))
                            .and_then(|node, act, ctx| {
                                fut::wrap_future::<_, Self>(
                                    node.unwrap().send(SendRemoteMessage(payload)),
                                )
                                .map_err(|err, _, _| panic!(err))
                                .and_then(|res, act, ctx| {
                                    fut::ok(handle_client_response(res, ctx, msg))
                                })
                            }),
                    )
                }),
        );
    }
}

fn handle_client_response(
    res: ClientResponseHandler,
    ctx: &mut Context<Raftor>,
    msg: ClientRequest,
) {
    match res {
        Ok(_) => (),
        Err(err) => match err {
            ClientError::Internal => {
                println!("TEST: resending client request.");
                ctx.notify(msg);
            }
            ClientError::Application(err) => {
                println!(
                    "Unexpected application error from client request: {:?}",
                    err
                );
            }
            ClientError::ForwardToLeader { .. } => {
                println!("TEST: received ForwardToLeader error. Updating leader and forwarding.");
                ctx.notify(msg);
            }
        },
    }
}
