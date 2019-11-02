use actix_raft::{messages::*, admin::InitWithConfig};

use crate::raft::storage::{
    MemoryStorageData,
    MemoryStorageError,
    MemoryStorageResponse
};
use crate::server::{GetMembers, CreateRoom, SendRecipient, SendRoom};
use crate::raftor::Raftor;


impl Raftor {
    fn register_handlers(&mut self) {
        // register raft handlers
        self.registry.register::<AppendEntriesRequest<MemoryStorageData>>(self.raft.recipient());
        self.registry.register::<VoteRequest>(self.raft.recipient());
        self.registry.register::<InitWithConfig>(self.raft.recipient());
        self.registry.register::<InstallSnapshotRequest>(self.raft.recipient());
        self.registry.register::<ClientPayload<MemoryStorageData, MemoryStorageResponse, MemoryStorageError>>(self.raft.recipient());

        // register server handlers
        self.registry.register::<GetMembers>(self.server.recipient());
        self.registry.register::<CreateRoom>(self.server.recipient());
        self.registry.register::<SendRoom>(self.server.recipient());
        self.registry.register::<SendRecipient>(self.server.recipient());
    }
}
