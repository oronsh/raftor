use actix::prelude::*;
use actix_raft::{RaftNetwork, messages};

use crate::network::{Network};
use crate::data::Data;

impl RaftNetwork<Data> for Network {}

impl Handler<messages::AppendEntriesRequest<Data>> for Network {
    type Result = std::result::Result<(), ()>;

    fn handle(&mut self, msg: messages::AppendEntriesRequest<Data>, ctx: &mut Context<Self>) {
        unimplemented!();
    }
}

impl Handler<messages::VoteRequest> for Network {
    type Result = std::result::Result<(), ()>;

    fn handle(&mut self, msg: messages::VoteRequest, ctx: &mut Context<Self>) {
        unimplemented!();
    }
}

impl Handler<messages::InstallSnapshotRequest> for Network {
    type Result = std::result::Result<(), ()>;

    fn handle(&mut self, msg: messages::InstallSnapshotRequest, ctx: &mut Context<Self>) {
        unimplemented!();
    }
}
