use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct U10(u16);

impl U10 {
    pub const BITS: usize = 10;
    pub const MAX: u16 = 0b11111_11111;
    pub const MIN: u16 = 0;

    pub fn new(value: u16) -> Option<Self> {
        if value > Self::MAX {
            None
        } else {
            Some(Self(value))
        }
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

pub fn current_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_millis()
}
