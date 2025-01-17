//! POSIX time to/from slot conversions

use crate::chain_query::EraSummary;
use crate::error::{Error, Result};
use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use plutus_ledger_api::{
    csl::lib as csl,
    v3::{
        interval::{Extended, LowerBound, PlutusInterval, UpperBound},
        transaction::{POSIXTime, POSIXTimeConversionError, POSIXTimeRange},
    },
};

/// Convert a POSIX time into number of slots in the current system
pub fn posix_time_into_slot(
    era_summaries: &[EraSummary],
    sys_start: &DateTime<Utc>,
    posix_time: POSIXTime,
) -> Result<u64> {
    let abs_time: DateTime<Utc> = posix_time
        .try_into()
        .map_err(|source: POSIXTimeConversionError| Error::ConversionError(anyhow!(source)))?;
    time_into_slot(era_summaries, sys_start, abs_time)
}

pub fn time_into_slot(
    era_summaries: &[EraSummary],
    sys_start: &DateTime<Utc>,
    time: DateTime<Utc>,
) -> Result<u64> {
    // Time since system start
    let rel_time = time - sys_start;

    if era_summaries.is_empty() {
        return Err(Error::InvalidConfiguration("era_summaries".to_string()));
    }

    let era_summary = era_summaries
        .iter()
        .filter(|era_summary| era_summary.start.time <= rel_time)
        .last()
        .ok_or(Error::InvalidPOSIXTime(format!(
            "{} is before system start at {}, or era_summaries are invalid.",
            time, sys_start
        )))?;

    let time_in_era = rel_time - era_summary.start.time;
    let slots_in_era = time_in_era.num_milliseconds() as u64 / era_summary.parameters.slot_length;

    Ok(era_summary.start.slot + slots_in_era)
}

pub fn slot_into_posix_time(
    era_summaries: &[EraSummary],
    sys_start: &DateTime<Utc>,
    slot: u64,
) -> Result<POSIXTime> {
    Ok(slot_into_time(era_summaries, sys_start, slot)?.into())
}

pub fn slot_into_time(
    era_summaries: &[EraSummary],
    sys_start: &DateTime<Utc>,
    slot: u64,
) -> Result<DateTime<Utc>> {
    let era_summary = era_summaries
        .iter()
        .filter(|era_summary| era_summary.start.slot <= slot)
        .last()
        .ok_or(Error::InvalidConfiguration("era_summaries".to_string()))?;

    let time_in_era = Duration::try_milliseconds(
        ((slot - era_summary.start.slot) * era_summary.parameters.slot_length) as i64,
    )
    .ok_or(Error::Internal(anyhow!(
        "Couldn't convert era in time to milliseconds"
    )))?;

    Ok(*sys_start + era_summary.start.time + time_in_era)
}

pub fn time_range_into_slots(
    era_summaries: &[EraSummary],
    sys_start: &DateTime<Utc>,
    interval: POSIXTimeRange,
) -> Result<(Option<csl::BigNum>, Option<csl::BigNum>)> {
    let PlutusInterval {
        from:
            LowerBound {
                bound: lower_bound,
                closed: lower_closed,
            },
        to: UpperBound {
            bound: upper_bound,
            closed: upper_closed,
        },
    } = interval;

    let lb_slots = match lower_bound {
        Extended::NegInf => None,
        Extended::PosInf => Some(u64::MAX),
        Extended::Finite(posix_time) => {
            let slots = posix_time_into_slot(era_summaries, sys_start, posix_time)?;
            if lower_closed {
                Some(slots)
            } else {
                Some(slots + 1)
            }
        }
    };

    let ub_slots = match upper_bound {
        Extended::NegInf => Some(0),
        Extended::PosInf => None,
        Extended::Finite(posix_time) => {
            let slots = posix_time_into_slot(era_summaries, sys_start, posix_time)?;
            if upper_closed {
                Some(slots)
            } else {
                Some(slots - 1)
            }
        }
    };

    Ok((
        lb_slots.map(csl::BigNum::from),
        ub_slots.map(csl::BigNum::from),
    ))
}
