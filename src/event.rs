use std::cmp::Ordering;

use chrono::{DateTime, Duration, TimeZone, Utc};
use rrule::RRuleSet;

#[derive(Debug)]
pub struct Event {
    pub id: String,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub rrule: Option<RRuleSet>,
}

impl Event {
    pub fn default() -> Event {
        Event {
            id: "".to_string(),
            summary: None,
            location: None,
            description: None,
            start: Utc.timestamp(0, 0),
            end: Utc.timestamp(0, 0),
            rrule: None,
        }
    }

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        let ord = self.start.cmp(&other.start);
        if ord == Ordering::Equal {
            self.end.cmp(&other.end)
        } else {
            ord
        }
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Event {}
