use rrule::RRuleSet;
use std::cmp::{Ordering, Ord};
use chrono::{DateTime, Duration, TimeZone, Utc};

#[derive(Debug)]
pub struct Event {
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub rrule: Option<RRuleSet>
}

impl Event {
    pub fn default() -> Event {
        Event {
            summary: None,
            location: None,
            description: None,
            start: Utc.timestamp(0, 0),
            end: Utc.timestamp(0, 0),
            rrule: None
        }
    }

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    pub fn id(&self) -> String {
        format!("{}{}{}", self.start, self.end,
                self.summary.as_ref().unwrap_or(&"<none>".to_string()))
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
