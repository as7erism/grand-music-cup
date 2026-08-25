use std::{
    sync::atomic::{AtomicU16, Ordering},
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use grand_music_cup::U10;

const TIMESTAMP_BITS: usize = 41;
const SNOWFLAKE_COUNTER_BITS: usize = 12;
const SNOWFLAKE_COUNTER_MAX: u16 = 0xFFF;

static SNOWFLAKE_COUNTER: AtomicU16 = AtomicU16::new(0);

pub struct SnowflakeManager {
    epoch: DateTime<Utc>,
    machine_id: U10,
}

impl SnowflakeManager {
    pub const fn new(epoch: DateTime<Utc>, machine_id: U10) -> Self {
        if epoch > DateTime::<Utc>::from(SystemTime::now()) {
            panic!("given epoch should be before the current time");
        }

        Self { epoch, machine_id }
    }

    pub fn make_snowflake(&self) -> i64 {
        // we want to calculate the timestamp after updating (via Ordering::Acquire) to help
        // mitigate collision. hopefully this never comes up :3
        let counter = SNOWFLAKE_COUNTER.update(Ordering::Acquire, Ordering::Relaxed, |c| {
            if c == SNOWFLAKE_COUNTER_MAX { 0 } else { c + 1 }
        });

        let timestamp = DateTime::<Utc>::from(SystemTime::now()).timestamp_millis()
            - self.epoch.timestamp_millis();
        if timestamp <= 0 {
            panic!("given epoch should be before the current time");
        }

        if timestamp >= 1i64 << TIMESTAMP_BITS {
            panic!(
                "timestamp cannot fit in {TIMESTAMP_BITS} bits! it may be time for a new epoch..."
            );
        }

        let mut value = timestamp << (U10::BITS + SNOWFLAKE_COUNTER_BITS);
        value |= (self.machine_id.as_u16() << SNOWFLAKE_COUNTER_BITS) as i64;
        value |= counter as i64;
        value
    }

    pub fn parse_timestamp(&self, snowflake: i64) -> Option<DateTime<Utc>> {
        DateTime::<Utc>::from_timestamp_millis(
            (snowflake & i64::MAX) >> (U10::BITS + SNOWFLAKE_COUNTER_BITS),
        )
    }

    pub fn parse_machine_id(&self, snowflake: i64) -> U10 {
        const MASK: i64 = (U10::MAX as i64) << SNOWFLAKE_COUNTER_BITS;
        U10::new(((snowflake & MASK) >> SNOWFLAKE_COUNTER_BITS) as u16)
            .expect("we should have masked out any extra bits")
    }

    pub fn parse_counter(&self, snowflake: i64) -> u16 {
        (snowflake & SNOWFLAKE_COUNTER_MAX as i64) as u16
    }
}
