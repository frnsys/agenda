use thiserror::Error;
use chrono_tz::{Tz, UTC};
use chrono::{DateTime, Date, TimeZone, Utc};
use std::cmp::{Ordering, Ord};

#[derive(Error, Debug)]
pub enum MomentError {
    #[error("Failed to parse date component")]
    ParseDateComponent(#[from] std::num::ParseIntError),

    #[error("Failed to parse datetime")]
    ParseDateTime(#[from] chrono::format::ParseError),
}


#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Moment {
    DateTime(DateTime<Utc>),
    Date(Date<Utc>)
}


impl Moment {
    pub fn parse(datetime: &str, time_zone: Option<String>) -> Result<Moment, MomentError> {
        let is_utc = datetime.chars().last().unwrap() == 'Z';
        let tz: Tz = if is_utc {
            UTC
        } else {
            match time_zone {
                Some(tz) => tz.parse().unwrap(),
                None => UTC // default to UTC
            }
        };
        if datetime.contains("T") {
            let fmt = if is_utc {
                "%Y%m%dT%H%M%SZ"
            } else {
                "%Y%m%dT%H%M%S"
            };
            let dt: DateTime<Utc> = tz.datetime_from_str(datetime, fmt)?.with_timezone(&Utc);
            Ok(Moment::DateTime(dt))
        } else {
            let d: Date<Utc> = tz.ymd(
                datetime[0..4].parse()?,
                datetime[4..6].parse()?,
                datetime[6..8].parse()?
            ).with_timezone(&Utc);
            Ok(Moment::Date(d))
        }
    }
}

impl Ord for Moment {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            Moment::DateTime(dt) => {
                match other {
                    Moment::DateTime(dt_) => dt.cmp(&dt_),
                    Moment::Date(d) => {
                        dt.date().cmp(d)
                    }
                }
            }
            Moment::Date(d) => {
                match other {
                    Moment::DateTime(dt) => {
                        d.cmp(&dt.date())
                    },
                    Moment::Date(d_) => d.cmp(&d_),
                }
            }
        }
    }
}

impl PartialOrd for Moment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Moment> for DateTime<Tz> {
    fn from(item: Moment) -> Self {
        match item {
            Moment::DateTime(dt) => dt.with_timezone(&UTC),
            Moment::Date(d) => d.and_hms(0, 0, 0).with_timezone(&UTC),
        }
    }
}

impl ToString for Moment {
    fn to_string(&self) -> String {
        match self {
            Moment::DateTime(dt) => dt.format("%Y%m%dT%H%M%SZ").to_string(),
            Moment::Date(d) => d.and_hms(0, 0, 0).format("%Y%m%dT%H%M%SZ").to_string()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_datetimes() {
        let d = "20211029";
        let expected = Moment::Date(Utc.ymd(2021, 10, 29));
        assert_eq!(Moment::parse(d, None).unwrap(), expected);

        let d = "20210524T024254Z";
        let expected = Moment::DateTime(Utc.ymd(2021, 5, 24).and_hms(02, 42, 54));
        assert_eq!(Moment::parse(d, None).unwrap(), expected);

        let d = "20200427T144500";
        let tz = "America/New_York".to_string();
        let expected = Moment::DateTime(Utc.ymd(2020, 4, 27).and_hms(18, 45, 00));
        assert_eq!(Moment::parse(d, Some(tz)).unwrap(), expected);
    }
}
