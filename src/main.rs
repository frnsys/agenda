mod ics;
mod event;

use std::fs;
use ics::parse_ics;
use anyhow::Error;
use event::Event;
use chrono::{DateTime, Date, Duration, Utc, Local, Datelike};
use chrono_tz::UTC;
use colored::*;
use std::collections::{HashSet,HashMap};
use notify_rust::Notification;

fn load_events() -> Result<Vec<Event>, Error>  {
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
    Ok(events)
}

fn load_upcoming_events(since: DateTime<Utc>, forecast: Duration) -> Result<Vec<Event>, Error> {
    let events = load_events()?;

    let end = since + forecast;
    let mut upcoming: Vec<Event> = events.into_iter().filter_map(|mut event| {
        match &event.rrule {
            Some(rrule) => {
                let next = rrule.after(since.with_timezone(&UTC), true);
                if let Some(next_occur) = next {
                    if next_occur <= end.with_timezone(&UTC) {
                        // Change event start to the next occurrence
                        let duration = event.duration();
                        event.start = next_occur.with_timezone(&Utc);
                        event.end = event.start + duration;
                        return Some(event)
                    }
                }
                None
            },
            None => {
                // TODO check end
                if event.start >= since && event.start <= end {
                    Some(event)
                } else {
                    None
                }
            }
        }
    }).collect();
    upcoming.sort();
    upcoming.dedup(); // TODO not sure why duplicates occur?
    Ok(upcoming)
}


/// View upcoming events for the next 5 days.
fn view() -> Result<(), Error> {
    // Treat "now" as the start of today (local time, but as UTC),
    // b/c if we're e.g. 1 minute into an event we still want to see it
    let forecast_days = 5;
    let now = Local::today().and_hms(0, 0, 0).with_timezone(&Utc);
    let upcoming = load_upcoming_events(now, Duration::days(forecast_days))?;

    let mut byday: HashMap<Date<Utc>, Vec<Event>> = HashMap::default();
    for event in upcoming {
        let events = byday.entry(event.start.date()).or_insert(vec![]);
        events.push(event);
    }

    for i in 0..forecast_days {
        let date = (now + Duration::days(i)).date();
        let date_str = date.format("%a %b %e").to_string().bold();
        if i == 0 {
            println!("{}\tToday", date_str);
        } else if i == 1 {
            println!("\n{}\tTomorrow", date_str);
        } else {
            println!("\n{}\t{} days", date_str, i);
        }
        match byday.get(&date) {
            Some(events) => {
                for event in events {
                    // Print out single event
                    let start_str = event.start.with_timezone(&Local).format("%a %b %e %H:%M");
                    let end_str_fmt = if event.start.day() == event.end.day() {
                        "%H:%M"
                    } else {
                        "%a %b %e %H:%M"
                    };
                    let end_str = event.end.with_timezone(&Local).format(end_str_fmt);
                    println!("{}-{}", start_str.to_string().green(), end_str.to_string().green());
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
                    println!("");
                }
            },
            None => {
                println!("No events");
            }
        }
    }
    Ok(())
}


/// Send a reminder for events starting in the next 10 minutes.
fn remind(reminded: &mut HashSet<String>) -> Result<(), Error> {
    let now = Utc::now();
    let upcoming = load_upcoming_events(now, Duration::minutes(10))?;
    for event in upcoming {
        let id = event.id();
        if !reminded.contains(&id) {
            Notification::new()
                .summary(&event.start.with_timezone(&Local).format("%H:%M").to_string())
                .body(&event.summary.unwrap_or("<none>".to_string()))
                .show()?;
            reminded.insert(id);
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let cmd = std::env::args().nth(1).expect("No command specified. Use 'view' or 'remind'.");

    match cmd.as_str() {
        "view" => view()?,
        "remind" => {
            let mut reminded = HashSet::new();
            loop {
                remind(&mut reminded)?;
                std::thread::sleep(std::time::Duration::new(60, 0));
            }
        },
        _ => {
            println!("Unrecognized command. Use 'view' or 'remind'");
        }
    }

    Ok(())
}
