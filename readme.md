# Usage

```
# See agenda for next 5 days
agenda view 5

# Send reminder notifications for events starting in the next 10 minutes
agenda remind 10
```
# Config

Create a configuration directory at `~/.config/agenda` and then create a file called `~/.config/agenda/calendars`.

The contents of this file should have a list of calendar names and ICS urls, separated by `;`. For example:

```
personal;https://calendar.google.com/calendar/ical/some/long/url/basic.ics
work;https://calendar.google.com/calendar/ical/some/other/url/basic.ics
```
