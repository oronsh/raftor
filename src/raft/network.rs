use actix::prelude::*;
use actix_raft::{RaftNetwork, messages};

use crate::network::{Network};
use crate::data::Data;

impl RaftNetwork<Data> for Network {}

impl Handler<messages::AppendEntriesRequest<Data>> for Network {
    type Result = ResponseActFuture<Self, messages::AppendEntriesResponse, ()>;

    fn handle(&mut self, msg: messages::AppendEntriesRequest<Data>, ctx: &mut Context<Self>) -> Self::Result {
        unimplemented!();
    }
}

impl Handler<messages::VoteRequest> for Network {
    type Result = ResponseActFuture<Self, messages::VoteResponse, ()>;

    fn handle(&mut self, msg: messages::VoteRequest, ctx: &mut Context<Self>) -> Self::Result {
        unimplemented!();
    }
}

impl Handler<messages::InstallSnapshotRequest> for Network {
    type Result = ResponseActFuture<Self, messages::InstallSnapshotResponse, ()>;

    fn handle(&mut self, msg: messages::InstallSnapshotRequest, ctx: &mut Context<Self>) -> Self::Result {
        unimplemented!();
    }
}
