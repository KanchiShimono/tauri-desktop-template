pub trait IdGenerator<Id>: Send + Sync {
    fn generate(&self) -> Id;
}
