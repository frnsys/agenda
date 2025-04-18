mod event;
mod ics;

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};

use ansi_term::{Color, Style};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Utc};
use chrono_tz::UTC;
use event::Event;
use expanduser::expanduser;
use fs_err::{self as fs, File};
use ics::parse_ics;

const FORECAST_DAYS: i64 = 5;
const REMINDER_MINUTES: i64 = 10;
const REMINDER_REFRESH: u64 = 120; // seconds
const UPDATE_EVERY: u64 = 5; // update every n reminder refresh intervals

/// Read all events from local ics files.
fn load_events() -> Result<Vec<Event>> {
    let mut events = vec![];
    let cals_path = expanduser("~/.config/agenda")?;
    for path in fs::read_dir(cals_path)? {
        let path = path?.path();
        if let Some(ext) = path.extension() {
            if ext == "ics" {
                events.extend(parse_ics(path)?);
            }
        }
    }
    Ok(events)
}

/// Load upcoming events relative to `since`, within `horizon`.
fn load_upcoming_events(since: DateTime<Utc>, horizon: Duration) -> Result<Vec<Event>> {
    let events = load_events()?;

    let end = since + horizon;
    let mut upcoming: Vec<Event> = events
        .into_iter()
        .filter_map(|mut event| {
            match &event.rrule {
                Some(rrule) => {
                    let next = rrule.after(since.with_timezone(&UTC), true);
                    if let Some(next_occur) = next {
                        if next_occur <= end.with_timezone(&UTC) {
                            // Change event start to the next occurrence
                            let duration = event.duration();
                            event.start = next_occur.with_timezone(&Utc);
                            event.end = event.start + duration;
                            return Some(event);
                        }
                    }
                    None
                }
                None => {
                    // TODO check end
                    if event.start >= since && event.start <= end {
                        Some(event)
                    } else {
                        None
                    }
                }
            }
        })
        .collect();
    upcoming.sort();
    upcoming.dedup();
    Ok(upcoming)
}

/// View upcoming events for the next `days` days.
fn view(days: i64) -> Result<()> {
    // Treat "now" as the start of today (local time, but as UTC),
    // b/c if we're e.g. 1 minute into an event we still want to see it
    let now = Local::now()
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).expect("Valid"))
        .unwrap();
    let upcoming = load_upcoming_events(now.with_timezone(&Utc), Duration::days(days))?;

    let mut byday: HashMap<NaiveDate, Vec<Event>> = HashMap::default();
    for event in upcoming {
        let events = byday
            .entry(event.start.with_timezone(&Local).date_naive())
            .or_default();
        events.push(event);
    }

    let date_style = Style::new()
        .on(Color::RGB(36, 34, 186))
        .fg(Color::RGB(255, 255, 255));
    let summary_style = Style::new().underline();
    let desc_style = Style::new().fg(Color::RGB(191, 190, 212));
    for i in 0..days {
        let date = (now + Duration::days(i)).date_naive();
        let date_str = date.format("%a %b %e").to_string();
        let date_str = if i == 0 {
            format!("{}\tToday", date_str)
        } else if i == 1 {
            format!("{}\tTomorrow", date_str)
        } else {
            format!("{}\t{} days", date_str, i)
        };
        println!("{}", date_style.paint(date_str));

        match byday.get(&date) {
            Some(events) => {
                for event in events {
                    // Print out single event
                    if (event.end - event.start).num_hours() == 24 {
                        println!("{}", Color::Green.paint("All Day"));
                    } else {
                        let start_str = event.start.with_timezone(&Local).format("%H:%M");
                        let end_str_fmt = if event.start.day() == event.end.day() {
                            "%H:%M"
                        } else {
                            "%a %b %e %H:%M"
                        };
                        let end_str = event.end.with_timezone(&Local).format(end_str_fmt);
                        println!(
                            "{} - {}",
                            Color::Green.paint(start_str.to_string()),
                            Color::Green.paint(end_str.to_string())
                        );
                    }
                    if let Some(summary) = &event.summary {
                        println!("{}", summary_style.paint(summary));
                    }
                    if let Some(location) = &event.location {
                        println!("{}", location);
                    }
                    if let Some(description) = &event.description {
                        // Unescape line breaks...is this the best way to do it?
                        println!("{}", desc_style.paint(description.replace("\\n", "\n")));
                    }
                    println!();
                }
            }
            None => {
                println!("No events\n");
            }
        }
    }
    Ok(())
}

/// Send a reminder for events starting in the next n minutes.
fn remind(reminded: &mut HashSet<String>, remind_before: Duration) -> Result<()> {
    let now = Utc::now();
    let upcoming = load_upcoming_events(now, remind_before)?;
    for event in upcoming {
        if !reminded.contains(&event.id) {
            Command::new("notify-send")
                .arg(
                    event
                        .start
                        .with_timezone(&Local)
                        .format("%H:%M")
                        .to_string(),
                )
                .arg(event.summary.unwrap_or("<none>".to_string()))
                .spawn()?;
            reminded.insert(event.id);
        }
    }
    Ok(())
}

/// Download a file
fn download(url: &str, path: &Path) -> Result<()> {
    let resp = ureq::get(url)
        .call()
        .with_context(|| format!("Failed to GET from '{}'", &url))?;

    let mut file = File::create(path)?;
    std::io::copy(&mut resp.into_reader(), &mut file)
        .with_context(|| "Error while downloading and writing file")?;

    Ok(())
}

/// Re-download the iCal files.
fn refresh() -> Result<()> {
    let dir = expanduser("~/.config/agenda")?;
    let config_path = dir.join("calendars");
    let contents = fs::read_to_string(&config_path)?;
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }

        let (name, url) = line.split_once(';').unwrap();
        let path = dir.join(format!("{name}.ics"));
        download(url, &path)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    let cmd = args
        .nth(1)
        .expect("No command specified. Use 'view', 'refresh', or 'remind'.");

    match cmd.as_str() {
        "view" => {
            let days = args
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(FORECAST_DAYS);
            view(days)?
        }
        "remind" => {
            let mut reminded = HashSet::new();
            let remind_mins = args
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(REMINDER_MINUTES);
            let remind_before = Duration::minutes(remind_mins);
            let sleep_dur = std::time::Duration::new(REMINDER_REFRESH, 0);
            let mut refresh_count = 0;
            loop {
                refresh_count += 1;
                if refresh_count % UPDATE_EVERY == 0 {
                    refresh()?;
                    refresh_count = 0;
                }
                remind(&mut reminded, remind_before)?;
                std::thread::sleep(sleep_dur);
            }
        }
        "refresh" => {
            println!("Updating calendars...");
            refresh()?;
            println!("Calendars updated.");
        }
        _ => {
            println!("Unrecognized command. Use 'view', 'refresh', or 'remind'");
        }
    }

    Ok(())
}
