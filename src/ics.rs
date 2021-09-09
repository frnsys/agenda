use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use ical::IcalParser;
use rrule::RRuleSet;
use anyhow::Error;
use super::moment::Moment;
use super::event::Event;

// Pull out the timezone, if any, from property parameters
fn get_tz(maybe_params: &Option<Vec<(String, Vec<String>)>>) -> Option<String> {
    if let Some(params) = maybe_params {
        let maybe_tz = params.iter().find(|(name, _)| name == "TZID");
        if let Some((_, value)) = maybe_tz {
            return Some(value[0].clone())
        }
    }
    None
}

// TODO iterator of results
pub fn parse_ics<P>(ics_path: P) -> Result<Vec<Event>, Error> where P: AsRef<Path> {
    let file = File::open(ics_path)?;
    let buf = BufReader::new(file);
    let reader = IcalParser::new(buf);

    let mut events = Vec::new();
    for line in reader {
        for ev in line?.events {
            let mut event = Event::default();
            for prop in ev.properties {
                match prop.name.as_ref() {
                    "DESCRIPTION" => event.description = prop.value,
                    "SUMMARY" => event.summary = prop.value,
                    "LOCATION" => event.location = prop.value,
                    "DTSTART" => {
                        let dt_str = prop.value.unwrap();
                        event.start = Moment::parse(&dt_str, get_tz(&prop.params))?;
                    },
                    "DTEND" => {
                        let dt_str = prop.value.unwrap();
                        event.end = Moment::parse(&dt_str, get_tz(&prop.params))?;
                    },
                    "RRULE" => {
                        // Kind of hacky, but the `rrule` crate doesn't provide
                        // a cleaner way of mixing string parsing and manually setting options.
                        let dtstart = event.start.to_string();
                        let rrule_str = format!("DTSTART:{}\n{}", dtstart, prop.value.unwrap());
                        let rrule: RRuleSet = rrule_str.parse()?;
                        event.rrule = Some(rrule);
                    },
                    "EXDATE" => {
                        let dt_str = prop.value.unwrap();
                        let ex_date = Moment::parse(&dt_str, get_tz(&prop.params))?;
                        if let Some(rrule) = &mut event.rrule {
                            rrule.exdate(ex_date.into());
                        }
                    },
                    "RECURRENCE-ID" => {
                        // TODO how to link back to the original event?
                        let dt_str = prop.value.unwrap();
                        let r_date = Moment::parse(&dt_str, get_tz(&prop.params))?;
                        match &mut event.rrule {
                            Some(rrule) => rrule.rdate(r_date.into()),
                            None => {
                                // TODO the problem right now is these show up as events
                                // separate from the event that has the actual rrule attached to
                                // it.
                                // println!("NO MATCHING RULE");
                            }
                        }
                    },
                    _ => ()
                }
            }
            events.push(event);
        }
    }
    Ok(events)
}
