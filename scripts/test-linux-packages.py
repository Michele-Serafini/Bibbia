#!/usr/bin/env python3
"""Exercise package payloads, real GTK mouse UI, restart and removal.

Run under an X11 display (xvfb-run + dbus-run-session on CI).
--install-deb installs/removes only in a clean disposable container.
--installed-rpm tests an already installed RPM in a clean Fedora container.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import stat
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / 'gnome/Cargo.toml').read_text())['package']['version']
APP_ID = 'io.github.MicheleSerafini.Bibbia'
OUT = ROOT / 'artifacts/diagnostics'
OUT.mkdir(parents=True, exist_ok=True)

def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)

def payload(prefix):
    required = ['bin/bibbia', f'share/applications/{APP_ID}.desktop',
                f'share/metainfo/{APP_ID}.metainfo.xml',
                f'share/icons/hicolor/256x256/apps/{APP_ID}.png',
                'share/bibbia/fonts/AdwaitaSans-Regular.ttf',
                'share/bibbia/fonts/SourceSerif4-Regular.ttf',
                'share/doc/bibbia/LICENSE.cei2008.txt', 'share/doc/bibbia/LICENSE.Adwaita',
                'share/doc/bibbia/LICENSE.SourceSerif', 'share/doc/bibbia/THIRD_PARTY_NOTICES.md']
    for name in required:
        path = prefix / name
        assert path.is_file() and path.stat().st_mode & stat.S_IROTH, name
    assert os.access(prefix / 'bin/bibbia', os.X_OK)
    for name, meta in json.loads((prefix / 'share/bibbia/fonts/sources.json').read_text()).items():
        assert hashlib.sha256((prefix / 'share/bibbia/fonts' / name).read_bytes()).hexdigest() == meta['sha256']

def native_ui(prefix, work, name):
    payload(prefix)
    data = work / f'{name}-data'
    env = os.environ.copy()
    env.update(XDG_CACHE_HOME=str(work / 'cache'), XDG_CONFIG_HOME=str(work / 'config'),
               XDG_DATA_HOME=str(work / 'data'), BIBBIA_SMOKE_SCREENSHOTS=str(OUT),
               XDG_DATA_DIRS=str(prefix / 'share') + ':' + env.get('XDG_DATA_DIRS', '/usr/local/share:/usr/share'))
    env.pop('BIBBIA_ASSETS_DIR', None)
    binary = prefix / 'bin/bibbia'
    with (OUT / f'{name}-smoke.log').open('w') as log:
        run(binary, '--smoke-test', '--data-dir', data, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=90)
    with (OUT / f'{name}-reopen.log').open('w') as log:
        run(binary, '--smoke-reopen', '--data-dir', data, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=45)
    # Reopen the actual closed database independently of the GUI/worker.
    with sqlite3.connect(data / 'bible.sqlite3') as db:
        settings = {k: json.loads(v) for k, v in db.execute('SELECT key,payload FROM settings')}
        assert settings['language'] == 'en' and settings['fontSize'] == 28
        assert settings['theme'] == 'Dark' and settings['position']['edition'] == 'Martini 1781 (cattolica)'
        marks = [json.loads(v) for (v,) in db.execute('SELECT payload FROM annotations')]
        assert any(m['note'] == 'Nota GNOME: luce 🌅' and m['bookmark'] and any(r['underline'] for r in m['ranges']) for m in marks)
    print(f'{name}: installed fonts, real mouse selection, compact annotation/note popovers, search, CEI/extended books and restart verified', flush=True)
    return data

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install-deb', action='store_true')
    parser.add_argument('--installed-rpm', action='store_true')
    args = parser.parse_args()
    if not os.environ.get('DISPLAY'):
        raise SystemExit('Run inside xvfb-run or a disposable X11 display.')
    with tempfile.TemporaryDirectory(prefix='bibbia-package-test-') as temporary:
        work = Path(temporary)
        if args.installed_rpm:
            assert os.geteuid() == 0, 'Requires a clean Fedora container'
            prefix = Path('/usr')
            data = native_ui(prefix, work, 'fedora-rpm')
            retained = data / 'bible.sqlite3'
            before = hashlib.sha256(retained.read_bytes()).hexdigest()
            run('rpm', '-e', 'bibbia')
            assert not (prefix / 'bin/bibbia').exists()
            assert hashlib.sha256(retained.read_bytes()).hexdigest() == before
            print('Fedora RPM removal preserves personal data')
            return
        deb = ROOT / f'artifacts/linux/bibbia_{VERSION}-1_amd64.deb'
        rpm = ROOT / f'artifacts/linux/bibbia-{VERSION}-1.x86_64.rpm'
        assert deb.is_file() and rpm.is_file()
        run('dpkg-deb', '--extract', deb, work / 'deb')
        native_ui(work / 'deb/usr', work, 'deb-payload')
        if args.install_deb:
            assert os.geteuid() == 0 and not Path('/usr/bin/bibbia').exists(), 'Use a clean disposable container'
            try:
                run('dpkg', '--install', deb)
                run('dpkg', '--verify', 'bibbia')
                data = native_ui(Path('/usr'), work, 'deb-installed')
                before = hashlib.sha256((data / 'bible.sqlite3').read_bytes()).hexdigest()
                run('dpkg', '--install', deb)
                run('/usr/bin/bibbia', '--smoke-reopen', '--data-dir', data, timeout=45)
                run('dpkg', '--remove', 'bibbia')
                assert not Path('/usr/bin/bibbia').exists()
                assert hashlib.sha256((data / 'bible.sqlite3').read_bytes()).hexdigest() == before
            finally:
                if Path('/usr/bin/bibbia').exists():
                    run('dpkg', '--remove', 'bibbia')
            print('Debian install, same-version upgrade and removal preserve personal data')
        # Use a private RPM database and relocatable prefix, never the host DB.
        database = work / 'rpm-db'; database.mkdir()
        prefix = work / 'rpm/usr'
        rpm_args = ['rpm', '--dbpath', str(database)]
        run('rpmdb', '--dbpath', database, '--initdb')
        run(*rpm_args, '--install', '--nodeps', '--noscripts', '--notriggers', '--prefix', prefix, rpm)
        data = native_ui(prefix, work, 'rpm-relocated')
        before = hashlib.sha256((data / 'bible.sqlite3').read_bytes()).hexdigest()
        sentinel = prefix / 'share/doc/bibbia/user-file.txt'; sentinel.write_text('Not owned by the package')
        run(*rpm_args, '--upgrade', '--replacepkgs', '--nodeps', '--noscripts', '--notriggers', '--prefix', prefix, rpm)
        run(prefix / 'bin/bibbia', '--smoke-reopen', '--data-dir', data, timeout=45)
        run(*rpm_args, '--erase', '--noscripts', '--notriggers', 'bibbia')
        assert not (prefix / 'bin/bibbia').exists() and sentinel.read_text() == 'Not owned by the package'
        assert hashlib.sha256((data / 'bible.sqlite3').read_bytes()).hexdigest() == before
        print('Private RPM install, upgrade and removal preserve notes and unrelated files')

if __name__ == '__main__':
    main()
