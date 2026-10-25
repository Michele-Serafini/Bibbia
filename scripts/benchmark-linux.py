#!/usr/bin/env python3
"""Measure the real GNOME release app on an X11 desktop, not the core harness.

Wait for the first rendered window, settle, then sample resident/proportional/
private memory and idle CPU. Requires a display; use xvfb-run + dbus-run-session
and GSK_RENDERER=cairo for reproducible software-rendering CI measurements.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/bibbia')
parser.add_argument('--output', type=Path, default=ROOT / 'artifacts/diagnostics/linux-performance.json')
parser.add_argument('--samples', type=int, default=3)
parser.add_argument('--idle-seconds', type=float, default=3)
args = parser.parse_args()
assert os.environ.get('DISPLAY'), 'An X11 display is required'
assert args.samples >= 1 and args.idle_seconds >= 1
rows = []
for sample in range(args.samples):
    with tempfile.TemporaryDirectory(prefix='bibbia-gnome-benchmark-') as temporary:
        work = Path(temporary)
        ready = work / 'ready'
        env = os.environ.copy()
        env.update(BIBBIA_BENCHMARK_READY=str(ready), XDG_CACHE_HOME=str(work / 'cache'),
                   XDG_CONFIG_HOME=str(work / 'config'), XDG_DATA_HOME=str(work / 'data'))
        started = time.perf_counter()
        with (work / 'stderr.log').open('w') as log:
            process = subprocess.Popen([str(args.binary.resolve()), '--data-dir', str(work / 'notes')], env=env, stdout=log, stderr=log)
            try:
                while not ready.exists():
                    assert process.poll() is None, (work / 'stderr.log').read_text()
                    assert time.perf_counter() - started < 30, 'Window did not become ready'
                    time.sleep(0.01)
                startup = (time.perf_counter() - started) * 1000
                time.sleep(1.5)
                def cpu():
                    fields = Path(f'/proc/{process.pid}/stat').read_text().rsplit(')', 1)[1].split()
                    return int(fields[11]) + int(fields[12])
                ticks = cpu(); before = time.perf_counter()
                time.sleep(args.idle_seconds)
                elapsed = time.perf_counter() - before
                percent = (cpu() - ticks) / os.sysconf('SC_CLK_TCK') / elapsed * 100
                memory = {}
                for line in Path(f'/proc/{process.pid}/smaps_rollup').read_text().splitlines():
                    if ':' in line:
                        key, value = line.split(':', 1)
                        memory[key] = int(value.split()[0])
                rows.append(dict(startup_ms=round(startup, 2), rss_mib=round(memory['Rss']/1024, 2),
                                 pss_mib=round(memory['Pss']/1024, 2),
                                 private_mib=round((memory['Private_Clean']+memory['Private_Dirty'])/1024, 2),
                                 idle_cpu_percent=round(percent, 3)))
            finally:
                process.terminate()
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
result = dict(context={'platform':platform.platform(), 'renderer':os.environ.get('GSK_RENDERER','default'),
                       'backend':os.environ.get('GDK_BACKEND','default'), 'edition':'Martini 1781 (cattolica)',
                       'notes':'Fresh private profiles, first chapter, no pending searches or selections. Includes GTK/libadwaita and fonts. CPU percent is one core. Xvfb/display-server memory excluded; this is not a Windows benchmark.'},
              samples=rows, median={key:statistics.median(row[key] for row in rows) for key in rows[0]})
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
