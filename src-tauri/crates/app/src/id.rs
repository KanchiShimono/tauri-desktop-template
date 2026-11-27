use uuid::Uuid;

use super::port::IdGenerator;

pub(crate) struct UuidV7Generator;

impl<Id: From<Uuid>> IdGenerator<Id> for UuidV7Generator {
    fn generate(&self) -> Id {
        Id::from(Uuid::now_v7())
    }
}
