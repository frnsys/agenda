mod event;
mod ics;

use ansi_term::{Color, Style};
use anyhow::Error;
use chrono::{Date, DateTime, Datelike, Duration, Local, Utc};
use chrono_tz::UTC;
use event::Event;
use expanduser::expanduser;
use ics::parse_ics;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

const FORECAST_DAYS: i64 = 5;
const REMINDER_MINUTES: i64 = 10;
const REMINDER_REFRESH: u64 = 120; // seconds

fn load_events() -> Result<Vec<Event>, Error> {
    let mut events = vec![];
    let cals_path = expanduser("~/.config/agenda").unwrap();
    println!("cals_path {:?}", cals_path);
    for path in fs::read_dir(&cals_path)? {
        let path = path?.path();
        if let Some(ext) = path.extension() {
            if ext == "ics" {
                events.extend(parse_ics(path)?);
            }
        }
    }
    Ok(events)
}

fn load_upcoming_events(since: DateTime<Utc>, forecast: Duration) -> Result<Vec<Event>, Error> {
    let events = load_events()?;

    let end = since + forecast;
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
    upcoming.dedup(); // TODO not sure why duplicates occur?
    Ok(upcoming)
}

/// View upcoming events for the next 5 days.
fn view(days: i64) -> Result<(), Error> {
    // Treat "now" as the start of today (local time, but as UTC),
    // b/c if we're e.g. 1 minute into an event we still want to see it
    let now = Local::today().and_hms(0, 0, 0);
    let upcoming = load_upcoming_events(now.with_timezone(&Utc), Duration::days(days))?;

    let mut byday: HashMap<Date<Local>, Vec<Event>> = HashMap::default();
    for event in upcoming {
        let events = byday
            .entry(event.start.with_timezone(&Local).date())
            .or_insert(vec![]);
        events.push(event);
    }

    let date_style = Style::new()
        .on(Color::RGB(36, 34, 186))
        .fg(Color::RGB(255, 255, 255));
    let summary_style = Style::new().underline();
    let desc_style = Style::new().fg(Color::RGB(191, 190, 212));
    for i in 0..days {
        let date = (now + Duration::days(i)).date();
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
                    // println!("{}", Color::Red.paint(&event.id));
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
fn remind(reminded: &mut HashSet<String>, remind_before: Duration) -> Result<(), Error> {
    let now = Utc::now();
    let upcoming = load_upcoming_events(now, remind_before)?;
    for event in upcoming {
        if !reminded.contains(&event.id) {
            Command::new("notify-send")
                .arg(
                    &event
                        .start
                        .with_timezone(&Local)
                        .format("%H:%M")
                        .to_string(),
                )
                .arg(&event.summary.unwrap_or("<none>".to_string()))
                .spawn()?;
            reminded.insert(event.id);
        }
    }
    Ok(())
}

/// Download a file
fn download(url: &str, path: &Path) -> Result<(), String> {
    let resp = ureq::get(url)
        .call()
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let mut file =
        File::create(path).or(Err(format!("Failed to create file '{}'", path.display())))?;
    std::io::copy(&mut resp.into_reader(), &mut file)
        .or(Err("Error while downloading and writing file"))?;

    Ok(())
}

/// Re-download the iCal files.
fn refresh() {
    let config_path = expanduser("~/.config/agenda/calendars").unwrap();
    let contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| format!("Couldn't read file: {:?}", &config_path));
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }

        let (name, url) = line.split_once(';').unwrap();
        let path = expanduser(format!("~/.config/agenda/{}.ics", name)).unwrap();
        download(url, &path).unwrap();
    }
}

fn main() -> Result<(), Error> {
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
            loop {
                remind(&mut reminded, remind_before)?;
                std::thread::sleep(sleep_dur);
            }
        }
        "refresh" => {
            println!("Updating calendars...");
            refresh();
            println!("Calendars updated.");
        }
        _ => {
            println!("Unrecognized command. Use 'view', 'refresh', or 'remind'");
        }
    }

    Ok(())
}
