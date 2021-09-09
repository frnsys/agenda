use rrule::RRuleSet;
use super::moment::Moment;
use std::cmp::{Ordering, Ord};
use chrono::{TimeZone, Utc};

#[derive(Debug)]
pub struct Event {
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: Moment,
    pub end: Moment,
    pub rrule: Option<RRuleSet>
}

impl Event {
    pub fn default() -> Event {
        Event {
            summary: None,
            location: None,
            description: None,
            start: Moment::DateTime(Utc.timestamp(0, 0)),
            end: Moment::DateTime(Utc.timestamp(0, 0)),
            rrule: None
        }
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
        self.summary == other.summary && self.start == other.start && self.end == other.end
    }
}
impl Eq for Event {}
