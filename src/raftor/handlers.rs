use actix_raft::{admin::InitWithConfig, messages::*};

use crate::raft::storage::{MemoryStorageData, MemoryStorageError, MemoryStorageResponse};
use crate::raftor::Raftor;
use crate::server::{CreateRoom, GetMembers, Join, SendRecipient, SendRoom};

impl Raftor {
    pub(crate) fn register_handlers(&mut self) {
        // register raft handlers
        let raft = self.raft.as_ref().unwrap();
        let mut registry = self.registry.write().unwrap();

        registry.register::<AppendEntriesRequest<MemoryStorageData>, _>(raft.clone());
        registry.register::<VoteRequest, _>(raft.clone());
        registry.register::<InstallSnapshotRequest, _>(raft.clone());
        registry.register::<ClientPayload<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>, _>(raft.clone());

        // register server handlers
        registry.register::<GetMembers, _>(self.server.clone());
        registry.register::<CreateRoom, _>(self.server.clone());
        registry.register::<SendRoom, _>(self.server.clone());
        registry.register::<SendRecipient, _>(self.server.clone());
        registry.register::<Join, _>(self.server.clone());
    }
}
