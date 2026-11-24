pub(crate) trait IdGenerator<Id>: Send + Sync {
    fn generate(&self) -> Id;
}
