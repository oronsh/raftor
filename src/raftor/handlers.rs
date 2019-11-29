
use crate::raftor::Raftor;
use crate::server::{CreateRoom, GetMembers, Join, SendRecipient, SendRoom};

impl Raftor {
    pub(crate) fn register_handlers(&mut self) {
        let mut registry = self.registry.write().unwrap();

        // register server handlers
        registry.register::<GetMembers, _>(self.server.clone());
        registry.register::<CreateRoom, _>(self.server.clone());
        registry.register::<SendRoom, _>(self.server.clone());
        registry.register::<SendRecipient, _>(self.server.clone());
        registry.register::<Join, _>(self.server.clone());
    }
}
