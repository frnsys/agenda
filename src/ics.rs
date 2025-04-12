use super::event::Event;
use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{Tz, UTC};
use fs_err::File;
use ical::IcalParser;
use rrule::RRuleSet;
use std::io::BufReader;
use std::path::Path;

// Pull out the timezone, if any, from property parameters
fn get_tz(maybe_params: &Option<Vec<(String, Vec<String>)>>) -> Option<String> {
    if let Some(params) = maybe_params {
        let maybe_tz = params.iter().find(|(name, _)| name == "TZID");
        if let Some((_, value)) = maybe_tz {
            return Some(value[0].clone());
        }
    }
    None
}

pub fn parse_datetime(datetime: &str, time_zone: Option<String>) -> Result<DateTime<Utc>> {
    let is_utc = datetime.ends_with('Z');
    let tz: Tz = if is_utc {
        UTC
    } else {
        match time_zone {
            Some(tz) => tz.parse().unwrap(),
            None => UTC, // default to UTC
        }
    };
    if datetime.contains("T") {
        let fmt = if is_utc {
            "%Y%m%dT%H%M%SZ"
        } else {
            "%Y%m%dT%H%M%S"
        };
        let dt = NaiveDateTime::parse_from_str(datetime, fmt)?
            .and_local_timezone(tz)
            .unwrap()
            .with_timezone(&Utc);
        Ok(dt)
    } else {
        // Handle these dates (which are all-day events)
        // as local timezone, so they properly span the full day.
        let dt = Local
            .with_ymd_and_hms(
                datetime[0..4].parse()?,
                datetime[4..6].parse()?,
                datetime[6..8].parse()?,
                0,
                0,
                0,
            )
            .unwrap()
            .with_timezone(&Utc);
        Ok(dt)
    }
}

/// Reconstruct raw DTSTART line for use in RRULE
pub fn reconstruct_datetime(datetime: &DateTime<Utc>, time_zone: Option<String>) -> String {
    let tz: Tz = match time_zone {
        Some(tz) => tz.parse().unwrap(),
        None => UTC, // default to UTC
    };
    let dt = datetime.with_timezone(&tz);
    if tz == UTC {
        dt.format(":%Y%m%dT%H%M%SZ").to_string()
    } else {
        dt.format(&format!(";TZID={}:%Y%m%dT%H%M%S", tz))
            .to_string()
    }
}

pub fn parse_ics<P>(ics_path: P) -> Result<Vec<Event>>
where
    P: AsRef<Path>,
{
    let file = File::open(ics_path.as_ref())?;
    let buf = BufReader::new(file);
    let reader = IcalParser::new(buf);

    let mut events: Vec<Event> = Vec::new();
    for line in reader {
        for ev in line?.events {
            let mut event = Event::default();
            let mut dtstart = None; // For RRULEs
            for prop in ev.properties {
                match prop.name.as_ref() {
                    "UID" => event.id = prop.value.unwrap(),
                    "DESCRIPTION" => event.description = prop.value,
                    "SUMMARY" => event.summary = prop.value,
                    "LOCATION" => event.location = prop.value,
                    "DTSTART" => {
                        let dt_str = prop.value.unwrap();
                        event.start = parse_datetime(&dt_str, get_tz(&prop.params))?;

                        // Reconstruct raw DTSTART line for use in RRULE
                        dtstart = Some(reconstruct_datetime(&event.start, get_tz(&prop.params)));
                    }
                    "DTEND" => {
                        let dt_str = prop.value.unwrap();
                        event.end = parse_datetime(&dt_str, get_tz(&prop.params))?;
                    }
                    "RRULE" => {
                        // Kind of hacky, but the `rrule` crate doesn't provide
                        // a cleaner way of mixing string parsing and manually setting options.
                        let rrule_str = format!(
                            "DTSTART{}\n{}",
                            dtstart.clone().unwrap(),
                            prop.value.unwrap()
                        );
                        let rrule: RRuleSet = rrule_str.parse()?;
                        event.rrule = Some(rrule);
                    }
                    "EXDATE" => {
                        let dt_str = prop.value.unwrap();
                        let ex_date = parse_datetime(&dt_str, get_tz(&prop.params))?;
                        if let Some(rrule) = &mut event.rrule {
                            rrule.exdate(ex_date.with_timezone(&UTC));
                        }
                    }
                    "RECURRENCE-ID" => {
                        match events.iter_mut().find(|ev| ev.id == event.id) {
                            Some(orig_event) => {
                                let dt_str = prop.value.unwrap();
                                let r_date = parse_datetime(&dt_str, get_tz(&prop.params))?;
                                match &mut orig_event.rrule {
                                    Some(rrule) => rrule.rdate(r_date.with_timezone(&UTC)),
                                    None => (), // Just treat it as its own event
                                }
                            }
                            None => (), // Just treat it as its own event
                        }
                    }
                    _ => (),
                }
            }
            events.push(event);
        }
    }
    Ok(events)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_datetimes() {
        let d = "20211029";
        let expected = Local
            .with_ymd_and_hms(2021, 10, 29, 0, 0, 0)
            .unwrap()
            .to_utc();
        assert_eq!(parse_datetime(d, None).unwrap(), expected);

        let d = "20210524T024254Z";
        let expected = Utc.with_ymd_and_hms(2021, 5, 24, 2, 42, 54).unwrap();
        assert_eq!(parse_datetime(d, None).unwrap(), expected);

        let d = "20200427T144500";
        let tz = "America/New_York".to_string();
        let expected = Utc.with_ymd_and_hms(2020, 4, 27, 18, 45, 00).unwrap();
        assert_eq!(parse_datetime(d, Some(tz)).unwrap(), expected);
    }
}
