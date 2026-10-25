#!/usr/bin/env python3
"""Exercise the relocated .app, native UI, persistence and idle resource use."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import sqlite3
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
DIAGNOSTICS = ROOT / 'artifacts/macos-diagnostics'


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def cpu_seconds(pid):
    raw = subprocess.check_output(['ps', '-p', str(pid), '-o', 'time='], text=True).strip()
    parts = raw.replace('-', ':').split(':')
    return sum(float(value) * factor for value, factor in zip(reversed(parts), [1, 60, 3600, 86400]))


def stop(process):
    if process.poll() is None:
        # SIGTERM is used only for disposable benchmark profiles. Integration
        # mode terminates through NSApplication and verifies its normal flush.
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def ui_process(binary, profile, mode, log_name, timeout):
    with (DIAGNOSTICS / log_name).open('w') as log:
        process = subprocess.Popen([str(binary), '--data-dir', str(profile), mode], stdout=log, stderr=log)
        try:
            status = process.wait(timeout=timeout)
            if status:
                raise RuntimeError(f'{mode} failed with exit status {status}')
        except subprocess.TimeoutExpired:
            subprocess.run(['sample', str(process.pid), '2', '10', '-file', str(DIAGNOSTICS / 'timeout-sample.txt')], timeout=15)
            raise
        finally:
            stop(process)


def benchmark(binary, root):
    samples = []
    for index in range(3):
        directory = root / f'benchmark-{index}'
        directory.mkdir()
        ready = directory / 'ready'
        env = dict(os.environ, BIBBIA_BENCHMARK_READY=str(ready))
        with (DIAGNOSTICS / f'benchmark-{index}.log').open('w') as log:
            started = time.monotonic()
            process = subprocess.Popen([str(binary), '--data-dir', str(directory)], env=env, stdout=log, stderr=log)
            try:
                while not ready.exists():
                    if process.poll() is not None or time.monotonic() - started > 30:
                        raise RuntimeError('Native app did not render its initial window')
                    time.sleep(0.02)
                startup = (time.monotonic() - started) * 1000
                time.sleep(1.5)
                rss = int(subprocess.check_output(['ps', '-p', str(process.pid), '-o', 'rss='], text=True)) / 1024
                before = cpu_seconds(process.pid)
                idle_started = time.monotonic()
                time.sleep(3)
                elapsed = time.monotonic() - idle_started
                cpu = (cpu_seconds(process.pid) - before) / elapsed * 100
                # AppKit/SwiftUI can finish deferred startup work after the
                # first frame. Keep the early sample and measure again later.
                time.sleep(5)
                steady_rss = int(subprocess.check_output(['ps', '-p', str(process.pid), '-o', 'rss='], text=True)) / 1024
                before = cpu_seconds(process.pid)
                idle_started = time.monotonic()
                time.sleep(3)
                elapsed = time.monotonic() - idle_started
                steady_cpu = (cpu_seconds(process.pid) - before) / elapsed * 100
                samples.append(dict(startup_ms=round(startup, 2), rss_mib=round(rss, 2),
                                    idle_cpu_percent=round(cpu, 2), steady_rss_mib=round(steady_rss, 2),
                                    steady_idle_cpu_percent=round(steady_cpu, 2)))
                if index == 2:
                    run('sample', process.pid, '2', '10', '-file', DIAGNOSTICS / 'idle-sample.txt', stdout=log, stderr=log)
            finally:
                stop(process)
    result = dict(platform=platform.platform(), architecture=platform.machine(),
                  method='Whole release app; 3 fresh profiles; ready after layout/display plus 100ms one-shot; early RSS/CPU after 1.5s settle over 3s; steady RSS/CPU after another 5s settle over 3s. ps RSS/CPU time, no input. Excludes WindowServer. Not a cross-machine comparison.',
                  samples=samples,
                  medians={key: round(statistics.median(sample[key] for sample in samples), 2) for key in samples[0]})
    (DIAGNOSTICS / 'performance.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


def exercise(app):
    DIAGNOSTICS.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='bibbia-macos-smoke-') as temporary:
        root = Path(temporary)
        applications = root / 'Applications with spaces'
        applications.mkdir()
        installed = applications / 'Bibbia.app'
        run('ditto', app, installed)
        run('codesign', '--verify', '--deep', '--strict', installed)
        binary = installed / 'Contents/MacOS/Bibbia'
        run('lipo', binary, '-verify_arch', 'arm64', 'x86_64')
        libraries = subprocess.check_output(['otool', '-L', str(binary)], text=True)
        print(libraries)
        dependencies = [line.strip().split(' (', 1)[0] for line in libraries.splitlines() if line.startswith('\t')]
        assert dependencies and all(path.startswith(('/usr/lib/', '/System/Library/')) for path in dependencies), libraries
        notices = list((installed / 'Contents/Resources').rglob('LICENSE.cei2008.txt'))
        assert len(notices) == 1 and notices[0].stat().st_size > 100
        profile = root / 'bibbia-macos-smoke-profile'
        profile.mkdir()
        (profile / '.bibbia-test-fixture').write_text('disposable integration fixture\n')
        try:
            ui_process(binary, profile, '--ui-smoke', 'ui.log', 60)
        finally:
            print((DIAGNOSTICS / 'ui.log').read_text())
            for screenshot in (profile / 'screenshots').glob('*.png'):
                shutil.copyfile(screenshot, DIAGNOSTICS / screenshot.name)
        assert (profile / 'ui-passed.txt').exists()
        ui_process(binary, profile, '--ui-reopen', 'reopen.log', 30)
        print((DIAGNOSTICS / 'reopen.log').read_text())
        assert (profile / 'reopen-passed.txt').exists()
        db = profile / 'bible.sqlite3'
        with sqlite3.connect(db) as connection:
            assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
            assert connection.execute('SELECT count(*) FROM annotations').fetchone()[0] > 0
        before = hashlib.file_digest(db.open('rb'), 'sha256').hexdigest()
        # Replacing the app leaves profiles alone; deleting it does so as well.
        shutil.rmtree(installed)
        run('ditto', app, installed)
        assert hashlib.file_digest(db.open('rb'), 'sha256').hexdigest() == before
        benchmark(binary, root)
        shutil.rmtree(installed)
        assert hashlib.file_digest(db.open('rb'), 'sha256').hexdigest() == before
        print('Relocated installation, app replacement/removal and personal-data preservation passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--app', type=Path)
    source.add_argument('--dmg', type=Path)
    args = parser.parse_args()
    if args.app:
        exercise(args.app.resolve())
    else:
        dmg = args.dmg.resolve()
        expected = dmg.with_name(dmg.name + '.sha256').read_text().split()[0]
        assert hashlib.file_digest(dmg.open('rb'), 'sha256').hexdigest() == expected
        run('hdiutil', 'verify', dmg)
        with tempfile.TemporaryDirectory(prefix='bibbia-macos-mount-') as mount:
            run('hdiutil', 'attach', '-readonly', '-nobrowse', '-mountpoint', mount, dmg)
            try:
                exercise(Path(mount) / 'Bibbia.app')
            finally:
                run('hdiutil', 'detach', mount)
