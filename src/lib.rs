use std::time::{SystemTime, UNIX_EPOCH};

use base64::prelude::*;
use rand::{
    SeedableRng, TryRng,
    rngs::{StdRng, SysRng},
};

const SECRET_LEN: usize = 32;
const MACHINE_ID: u16 = 0; // will need to figure out a real solution for this if this scales, lol

pub fn generate_secret() -> String {
    let mut buffer: [u8; _] = [0; SECRET_LEN];
    let mut rng = StdRng::try_from_rng(&mut SysRng).expect("seeding rng failed");
    rng.try_fill_bytes(&mut buffer)
        .expect("filling buffer with random bytes failed");
    BASE64_STANDARD.encode(buffer)
}

pub struct Snowflake(i64);

impl From<i64> for Snowflake {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<u64> for Snowflake {
   fn from(value: u64) -> Self {
        Self(value as i64)
    }
}

impl Snowflake {
    pub fn new(machine_id: U10) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("current time should be after unix epoch").as_millis()
        if timestamp > 0x1_FFFFF_FFFFF { // max 41 bit integer
            panic!("this code is apparently running in, like, the year 80,000");
        }

        
    }
}

pub struct U10(u16);

impl U10 {
    pub fn new(value: u16) -> Option<Self> {
        if value > 0b11111_11111 {
            None
        } else {
            Some(Self(value))
        }
    }
}
