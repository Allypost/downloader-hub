use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Timeframe {
    Nanoseconds(u64),
    Milliseconds(u64),
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
    Weeks(u64),
    Months(u64),
    Other(Duration),
}

impl From<Timeframe> for Duration {
    fn from(value: Timeframe) -> Self {
        (&value).into()
    }
}

impl From<&Timeframe> for Duration {
    fn from(val: &Timeframe) -> Self {
        match val {
            Timeframe::Nanoseconds(ns) => Self::from_nanos(*ns),
            Timeframe::Milliseconds(ms) => Self::from_millis(*ms),
            Timeframe::Seconds(s) => Self::from_secs(*s),
            Timeframe::Minutes(m) => Self::from_secs(*m * 60),
            Timeframe::Hours(h) => Self::from_secs(*h * 60 * 60),
            Timeframe::Days(d) => Self::from_secs(*d * 24 * 60 * 60),
            Timeframe::Weeks(w) => Self::from_secs(*w * 7 * 24 * 60 * 60),
            Timeframe::Months(m) => Self::from_secs(*m * 30 * 24 * 60 * 60),
            Timeframe::Other(d) => d.to_owned(),
        }
    }
}

impl From<&Timeframe> for String {
    fn from(val: &Timeframe) -> Self {
        let dur: Duration = val.into();

        format!("r{secs}", secs = dur.as_secs())
    }
}

impl From<Timeframe> for String {
    fn from(val: Timeframe) -> Self {
        (&val).into()
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str: String = self.into();

        write!(f, "{}", str)
    }
}

#[derive(Debug, Clone)]
pub struct TimeframeParseError(String);
impl std::fmt::Display for TimeframeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TimeframeParseError {}

impl Timeframe {
    pub fn parse_str(arg: &str) -> Result<Self, TimeframeParseError> {
        let arg = arg.trim().to_lowercase();

        let num = arg
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();

        if num.is_empty() {
            return Err(TimeframeParseError(format!(
                "invalid timeframe (no number found): {arg}"
            )));
        }

        let unit = arg.chars().skip(num.len()).collect::<String>();

        let num = num.parse::<u64>().map_err(|_| {
            TimeframeParseError(format!("invalid timeframe (invalid number): {arg}"))
        })?;

        match unit.trim() {
            "mon" | "month" | "months" => Ok(Self::Months(num)),
            "w" | "week" | "weeks" => Ok(Self::Weeks(num)),
            "d" | "day" | "days" => Ok(Self::Days(num)),
            "h" | "hr" | "hrs" | "hour" | "hours" => Ok(Self::Hours(num)),
            "min" | "mins" | "minute" | "minutes" => Ok(Self::Minutes(num)),
            "s" | "sec" | "secs" | "second" | "seconds" => Ok(Self::Seconds(num)),
            "ms" | "msec" | "msecs" | "millisecond" | "milliseconds" => Ok(Self::Milliseconds(num)),
            "ns" | "nsec" | "nsecs" | "nanosecond" | "nanoseconds" => Ok(Self::Nanoseconds(num)),
            _ => Err(TimeframeParseError(format!(
                "invalid timeframe (invalid unit): {arg}"
            ))),
        }
    }
}
