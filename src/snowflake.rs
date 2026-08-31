use std::{
    sync::atomic::{AtomicU16, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use grand_music_cup::U10;
use thiserror::Error;
use time::UtcDateTime;

const TIMESTAMP_BITS: usize = 41;
const SNOWFLAKE_COUNTER_BITS: usize = 12;
const SNOWFLAKE_COUNTER_MAX: u16 = 0xFFF;

static SNOWFLAKE_COUNTER: AtomicU16 = AtomicU16::new(0);

// TODO revamp this api

#[derive(Debug, Error)]
pub enum SnowflakeError {
    #[error("epoch should not be in the future")]
    EpochInFuture,
    #[error("snowflake timestamp is out of range")]
    TimestampOutOfRange,
}

#[derive(Clone, Debug)]
pub struct Snowflake {
    epoch_ms: i64,
    value: i64,
}

impl Snowflake {
    pub fn new_unique(epoch: UtcDateTime, machine_id: U10) -> Result<Self, SnowflakeError> {
        if epoch > UtcDateTime::now() {
            Err(SnowflakeError::EpochInFuture)
        } else {
            Ok(Self::make_raw(epoch.unix_timestamp(), machine_id))
        }
    }

    fn make_raw(epoch_ms: i64, machine_id: U10) -> Self {
        // we want to calculate the timestamp after updating (via Ordering::Acquire) to help
        // mitigate collision. hopefully this never comes up :3
        let counter = SNOWFLAKE_COUNTER.update(Ordering::Acquire, Ordering::Relaxed, |c| {
            if c == SNOWFLAKE_COUNTER_MAX { 0 } else { c + 1 }
        });

        let timestamp = UtcDateTime::now().unix_timestamp() - epoch_ms;
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

    pub fn from_i64(epoch: UtcDateTime, value: i64) -> Result<Self, SnowflakeError> {
        if epoch > UtcDateTime::now() {
            Err(SnowflakeError::EpochInFuture)
        } else {
            let snowflake = Self {
                value,
                epoch_ms: epoch.unix_timestamp(),
            };
            snowflake.validate()?;
            Ok(Self {
                value,
                epoch_ms: epoch.unix_timestamp(),
            })
        }
    }

    pub fn as_i64(&self) -> i64 {
        self.value
    }

    fn timestamp_ms(&self) -> i64 {
        (self.value & i64::MAX) >> (U10::BITS + SNOWFLAKE_COUNTER_BITS)
    }

    fn validate(&self) -> Result<(), SnowflakeError> {
        let _ = UtcDateTime::from_unix_timestamp(
            self.timestamp_ms()
                .checked_add(self.epoch_ms)
                .ok_or(SnowflakeError::TimestampOutOfRange)?,
        )
        .map_err(|_| SnowflakeError::TimestampOutOfRange)?;
        Ok(())
    }

    /// The time at which this snowflake was generated
    pub fn timestamp(&self) -> UtcDateTime {
        UtcDateTime::from_unix_timestamp(self.timestamp_ms() + self.epoch_ms)
            .expect("we should have validated this")
    }

    pub fn machine_id(&self) -> U10 {
        const MASK: i64 = (U10::MAX as i64) << SNOWFLAKE_COUNTER_BITS;
        U10::new(((self.value & MASK) >> SNOWFLAKE_COUNTER_BITS) as u16)
            .expect("we should have masked out any extra bits")
    }

    pub fn counter(&self) -> u16 {
        (self.value & SNOWFLAKE_COUNTER_MAX as i64) as u16
    }

    pub fn epoch(&self) -> UtcDateTime {
        UtcDateTime::from_unix_timestamp(self.epoch_ms).expect("stored epoch should be in range")
    }
}
