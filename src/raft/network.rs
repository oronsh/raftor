use actix::prelude::*;
use actix_raft::{messages, RaftNetwork};
use log::error;

use crate::network::{remote::SendRemoteMessage, Network};
use crate::raft::storage::MemoryStorageData as Data;

const ERR_ROUTING_FAILURE: &str = "Failed to send RCP to node target.";

impl RaftNetwork<Data> for Network {}

impl Handler<messages::AppendEntriesRequest<Data>> for Network {
    type Result = ResponseActFuture<Self, messages::AppendEntriesResponse, ()>;

    fn handle(
        &mut self,
        msg: messages::AppendEntriesRequest<Data>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        let target_id = msg.target;
        if let Some(node) = self.get_node(msg.target) {

            if self.isolated_nodes.contains(&msg.target) || self.isolated_nodes.contains(&msg.leader_id) {
                return Box::new(fut::err(()));
            }

            let req = node.send(SendRemoteMessage(msg));

            return Box::new(
                fut::wrap_future(req)
                    .map_err(move |_, _, _| error!("{} {}", ERR_ROUTING_FAILURE, target_id))
                    .and_then(|res, _, _| fut::result(res)),
            );

        }

        Box::new(fut::err(()))
    }
}

impl Handler<messages::VoteRequest> for Network {
    type Result = ResponseActFuture<Self, messages::VoteResponse, ()>;

    fn handle(&mut self, msg: messages::VoteRequest, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(node) = self.get_node(msg.target) {


            if self.isolated_nodes.contains(&msg.target) || self.isolated_nodes.contains(&msg.candidate_id) {
                return Box::new(fut::err(()));
            }

            let req = node.send(SendRemoteMessage(msg));

            return Box::new(
                fut::wrap_future(req)
                    .map_err(|_, _, _| error!("{}", ERR_ROUTING_FAILURE))
                    .and_then(|res, _, _| fut::result(res)),
            );
        }

        Box::new(fut::err(()))
    }
}

impl Handler<messages::InstallSnapshotRequest> for Network {
    type Result = ResponseActFuture<Self, messages::InstallSnapshotResponse, ()>;

    fn handle(
        &mut self,
        msg: messages::InstallSnapshotRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        if let Some(node) = self.get_node(msg.target) {
            if self.isolated_nodes.contains(&msg.target) || self.isolated_nodes.contains(&msg.leader_id) {
                return Box::new(fut::err(()));
            }

            let req = node.send(SendRemoteMessage(msg));

            return Box::new(
                fut::wrap_future(req)
                    .map_err(|_, _, _| error!("{}", ERR_ROUTING_FAILURE))
                    .and_then(|res, _, _| fut::result(res)),
            );
        }

        Box::new(fut::err(()))
    }
}
