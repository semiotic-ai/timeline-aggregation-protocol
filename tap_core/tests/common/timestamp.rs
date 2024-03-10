use std::time::{SystemTime, UNIX_EPOCH};

use tap_core::Error;

pub fn get_current_timestamp_u64_ns() -> anyhow::Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}
