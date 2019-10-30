use actix_raft::{
    messages,
    admin,
};

use crate::raft::{
    storage
};
use crate::raftor::Raftor;

impl Raftor {
    fn register_recipient<M>(&mut self, recipient: Recipient<M>)
    where
        M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned,
        MemRaft: Handler<M>
    {
        let r = Provider{recipient};
        self.handlers.insert(M::type_id(), Arc::new(r));
    }
}
