use jiff::Timestamp;

use crate::port::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}
