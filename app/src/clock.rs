use chrono::{DateTime, Local, NaiveTime, Utc};

/// Midnight of the user's current local day, in UTC. "Today" is what the user calls today.
pub fn start_of_local_day() -> DateTime<Utc> {
    Local::now()
        .with_time(NaiveTime::MIN)
        .earliest()
        .map_or_else(Utc::now, |midnight| midnight.with_timezone(&Utc))
}
