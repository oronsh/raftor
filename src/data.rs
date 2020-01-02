use actix::prelude::*;
use evmap::{WriteHandle, ReadHandle}

use crate::{
    network::Network,
    raft::{RaftClient},
    channel::{Channel},
};

#[derive(Clone, Debug)]
pub struct ServerData {
    pub sessions_w: WriteHandle<String, Arc<Addr<Session>>>,
    pub sessions_r: ReadHandle<String, Arc<Addr<Session>>>,
    pub channels_w: WriteHandle<String, Arc<Addr<Channel>>>,
    pub channels_r: ReadHandle<String, Arc<Addr<Channel>>>,
    pub net: Addr<Network>,
    pub raft: Addr<RaftClient>,
}

impl ServerData {
    pub fn new(net: Addr<Network>, raft: Addr<RaftClient>) -> Self {
        let (sessions_r, mut sessions_w) = evmap::new::<String, Arc<Addr<Session>>>();
        let (channels_r, mut channels_w) = evmap::new::<String, Arc<Addr<Channel>>>();

        Self {
            sessions_w: sessions_w,
            sessions_r: sessions_r,
            channels_w: channels_w,
            channels_r: channels_r,
            net: net,
            raft: raft
        }
    }

    pub fn register_session(&self, addr: Addr<Session>) {
        let session = Arc::new(addr);
    }

    pub fn register_channel(&self) {

    }
}
