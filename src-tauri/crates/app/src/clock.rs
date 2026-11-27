use jiff::Timestamp;

use super::port::Clock;

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}
