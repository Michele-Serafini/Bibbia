#!/usr/bin/env python3
"""Validate native iCalendar exports without a calendar account or GUI app."""
from datetime import datetime, timedelta
from pathlib import Path
import sys

raw = Path(sys.argv[1]).read_bytes()
assert raw.endswith(b'END:VCALENDAR\r\n') and b'\n' not in raw.replace(b'\r\n', b'')
physical = raw.split(b'\r\n')[:-1]
assert all(len(line) <= 75 for line in physical)
assert all(line.decode('utf-8') is not None for line in physical)
unfolded = raw.replace(b'\r\n ', b'').replace(b'\r\n\t', b'').decode('utf-8').split('\r\n')
assert unfolded[0] == 'BEGIN:VCALENDAR' and 'VERSION:2.0' in unfolded
events, event = [], None
for line in unfolded:
    if line == 'BEGIN:VEVENT':
        assert event is None
        event = {}
    elif line == 'END:VEVENT':
        assert event is not None
        events.append(event)
        event = None
    elif event is not None:
        key, value = line.split(':', 1)
        assert key not in event
        event[key] = value
assert event is None and events
ids, dates = set(), []
for entry in events:
    assert entry['UID'] not in ids
    ids.add(entry['UID'])
    day = datetime.strptime(entry['DTSTART;VALUE=DATE'], '%Y%m%d')
    end = datetime.strptime(entry['DTEND;VALUE=DATE'], '%Y%m%d')
    assert end - day == timedelta(days=1)
    assert not dates or day - dates[-1] == timedelta(days=1)
    dates.append(day)
    datetime.strptime(entry['DTSTAMP'], '%Y%m%dT%H%M%SZ')
    assert entry['SUMMARY'] and entry['DESCRIPTION']
print(f'iCalendar verified: {len(events)} all-day events, exclusive end dates, unique IDs and valid UTF-8 folding.')
