use actix_raft::{admin::InitWithConfig, messages::*};

use crate::raft::storage::{MemoryStorageData, MemoryStorageError, MemoryStorageResponse};
use crate::raftor::Raftor;
use crate::server::{CreateRoom, GetMembers, Join, SendRecipient, SendRoom};

impl Raftor {
    pub(crate) fn register_handlers(&mut self) {
        // register raft handlers
        let raft = self.raft.as_ref().unwrap();
        let mut registry = self.registry.write().unwrap();

        registry.register::<AppendEntriesRequest<MemoryStorageData>>(raft.clone().recipient());
        registry.register::<VoteRequest>(raft.clone().recipient());
        registry.register::<InstallSnapshotRequest>(raft.clone().recipient());
        registry.register::<ClientPayload<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>>(raft.clone().recipient());

        // register server handlers
        registry.register::<GetMembers>(self.server.clone().recipient());
        registry.register::<CreateRoom>(self.server.clone().recipient());
        registry.register::<SendRoom>(self.server.clone().recipient());
        registry.register::<SendRecipient>(self.server.clone().recipient());
        registry.register::<Join>(self.server.clone().recipient());
    }
}
