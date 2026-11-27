use uuid::Uuid;

use crate::port::IdGenerator;

pub struct UuidV7Generator;

impl<Id: From<Uuid>> IdGenerator<Id> for UuidV7Generator {
    fn generate(&self) -> Id {
        Id::from(Uuid::now_v7())
    }
}
