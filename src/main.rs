mod ics;
mod event;
mod moment;

use std::fs;
use ics::parse_ics;
use moment::Moment;
use anyhow::Error;
use event::Event;
use chrono::{Duration, Utc, Local, Datelike};
use chrono_tz::UTC;
use colored::*;

fn main() -> Result<(), Error> {
    // Treat "now" as the start of today (local time, but as UTC),
    // b/c if we're e.g. 1 minute into an event we still want to see it
    let forecast_days = 5;
    let now = Utc::today().and_hms(0, 0, 0);
    let end = now + Duration::days(forecast_days);

    let mut events = vec![];
    for path in fs::read_dir("/home/ftseng/.calendar")? {
        for subpath in fs::read_dir(path?.path())? {
            let ics = subpath?.path();
            if let Some(ext) = ics.extension() {
                if ext == "ics" {
                    events.extend(parse_ics(ics)?);
                }
            }
        }
    }

    let mut upcoming: Vec<Event> = events.into_iter().filter_map(|mut event| {
        match &event.rrule {
            Some(rrule) => {
                let next = rrule.after(now.with_timezone(&UTC), true);
                if let Some(next_occur) = next {
                    if next_occur <= end.with_timezone(&UTC) {
                        // Change event start to the next occurrence
                        event.start = Moment::DateTime(next_occur.with_timezone(&Utc));
                        return Some(event)
                    }
                }
                None
            },
            None => {
                if match event.start {
                    Moment::DateTime(dt) => dt >= now && dt <= end,
                    Moment::Date(d) => d >= now.date() && d <= end.date()
                } {
                    Some(event)
                } else {
                    None
                }
            }
        }
    }).collect();
    upcoming.sort();

    println!("Today");
    for event in &upcoming {
        // let now = Utc::today().and_hms(0, 0, 0);
        // let end = now + Duration::days(forecast_days);
        let start_str = match event.start {
            Moment::DateTime(dt) => dt.with_timezone(&Local).format("%a %b %e %H:%M"),
            Moment::Date(d) => d.with_timezone(&Local).format("%a %b %e"),
        };
        println!("{}", start_str.to_string().bold());
        let end_str = match event.end {
            Moment::DateTime(dt) => dt.with_timezone(&Local).format("%a %b %e %H:%M"),
            Moment::Date(d) => d.with_timezone(&Local).format("%a %b %e"),
        };
        println!("{}", end_str.to_string().bold());
        if let Some(summary) = &event.summary {
            println!("{}", summary.yellow());
        }
        if let Some(location) = &event.location {
            println!("{}", location);
        }
        if let Some(description) = &event.description {
            // Unescape line breaks...is this the best way to do it?
            println!("{}", description.replace("\\n", "\n"));
        }
        // if (event.start.day() > now.day()) {
        //     now += Duration::days(1);
        // }
    }

    Ok(())
}
