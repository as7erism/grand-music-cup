use std::{
    sync::atomic::{AtomicU16, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use grand_music_cup::U10;
use thiserror::Error;

const TIMESTAMP_BITS: usize = 41;
const SNOWFLAKE_COUNTER_BITS: usize = 12;
const SNOWFLAKE_COUNTER_MAX: u16 = 0xFFF;

static SNOWFLAKE_COUNTER: AtomicU16 = AtomicU16::new(0);

// TODO revamp this api

#[derive(Debug, Error)]
pub enum SnowflakeError {
    #[error("epoch should not be in the future")]
    EpochInFuture,
}

#[derive(Clone, Debug)]
pub struct Snowflake {
    epoch_ms: u64,
    value: i64,
}

impl Snowflake {
    pub fn new_unique(epoch_ms: u64, machine_id: U10) -> Result<Self, SnowflakeError> {
        if epoch_ms
            > SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current time should be after unix epoch")
                .as_millis() as u64
        {
            Err(SnowflakeError::EpochInFuture)
        } else {
            Ok(Self::make_raw(epoch_ms, machine_id))
        }
    }

    fn make_raw(epoch_ms: u64, machine_id: U10) -> Self {
        // we want to calculate the timestamp after updating (via Ordering::Acquire) to help
        // mitigate collision. hopefully this never comes up :3
        let counter = SNOWFLAKE_COUNTER.update(Ordering::Acquire, Ordering::Relaxed, |c| {
            if c == SNOWFLAKE_COUNTER_MAX { 0 } else { c + 1 }
        });

        let timestamp = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_millis() as u64
            - epoch_ms) as i64;
        if timestamp <= 0 {
            panic!("given epoch should be before the current time");
        }

        if timestamp >= 1i64 << TIMESTAMP_BITS {
            panic!(
                "timestamp cannot fit in {TIMESTAMP_BITS} bits! it may be time for a new epoch..."
            );
        }

        let mut value = timestamp << (U10::BITS + SNOWFLAKE_COUNTER_BITS);
        value |= (machine_id.as_u16() << SNOWFLAKE_COUNTER_BITS) as i64;
        value |= counter as i64;
        Self { epoch_ms, value }
    }

    pub fn from_i64(value: i64, epoch_ms: u64) -> Result<Self, SnowflakeError> {
        if epoch_ms
            > SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current time should be after unix epoch")
                .as_millis() as u64
        {
            Err(SnowflakeError::EpochInFuture)
        } else {
            Ok(Self { epoch_ms, value })
        }
    }

    pub fn as_i64(&self) -> i64 {
        self.value
    }

    /// The time at which this snowflake was generated as milliseconds since `epoch_ms()`
    pub fn timestamp_ms(&self) -> u64 {
        ((self.value & i64::MAX) >> (U10::BITS + SNOWFLAKE_COUNTER_BITS)) as u64
    }

    pub fn machine_id(&self) -> U10 {
        const MASK: i64 = (U10::MAX as i64) << SNOWFLAKE_COUNTER_BITS;
        U10::new(((self.value & MASK) >> SNOWFLAKE_COUNTER_BITS) as u16)
            .expect("we should have masked out any extra bits")
    }

    pub fn counter(&self) -> u16 {
        (self.value & SNOWFLAKE_COUNTER_MAX as i64) as u16
    }

    pub fn epoch_ms(&self) -> u64 {
        self.epoch_ms
    }
}
