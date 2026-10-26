#!/usr/bin/env python3
"""Fail the build if native applications disagree on the release version."""
import json
import plistlib
import re
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    expected = (ROOT / 'VERSION').read_text().strip()
    if not re.fullmatch(r'\d+\.\d+\.\d+', expected):
        raise SystemExit('VERSION must contain a semantic release version')
    versions = {
        'Windows': ET.parse(ROOT / 'app/Bibbia.Windows.csproj').findtext('./PropertyGroup/Version'),
        'Rust core': tomllib.loads((ROOT / 'core/Cargo.toml').read_text())['package']['version'],
        'GNOME': tomllib.loads((ROOT / 'gnome/Cargo.toml').read_text())['package']['version'],
        'macOS': plistlib.loads((ROOT / 'macos/Info.plist').read_bytes())['CFBundleShortVersionString'],
        'GNOME AppStream': ET.parse(ROOT / 'gnome/data/io.github.MicheleSerafini.Bibbia.metainfo.xml').find('./releases/release').get('version'),
    }
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    for package in lock['package']:
        if package['name'] in ('bible-core', 'bibbia-gnome'):
            versions['Cargo.lock ' + package['name']] = package['version']
    mismatches = {name: version for name, version in versions.items() if version != expected}
    if mismatches:
        raise SystemExit(f'Expected {expected} on every platform: {json.dumps(mismatches)}')
    print(f'Bibbia {expected}: Windows, Linux and macOS versions agree')


if __name__ == '__main__':
    main()
