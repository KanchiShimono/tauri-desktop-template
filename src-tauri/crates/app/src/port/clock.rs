use jiff::Timestamp;

pub(crate) trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
