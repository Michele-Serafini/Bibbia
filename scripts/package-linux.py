#!/usr/bin/env python3
"""Build native, dynamically linked GNOME .deb and .rpm packages (x86-64).

Requires cargo, pkg-config, GTK4/libadwaita development packages, dpkg-deb,
rpmbuild, desktop-file-validate and appstream-util. No external packaging SDK.
"""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
APP_ID = 'io.github.MicheleSerafini.Bibbia'
VERSION = tomllib.loads((ROOT / 'gnome/Cargo.toml').read_text())['package']['version']

def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)

def copy(source, target, executable=False):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    target.chmod(0o755 if executable else 0o644)

def payload(directory, binary):
    copy(binary, directory / 'usr/bin/bibbia', True)
    for source in (ROOT / 'assets/fonts').iterdir():
        if source.is_file():
            copy(source, directory / 'usr/share/bibbia/fonts' / source.name)
    copy(ROOT / 'gnome/data' / f'{APP_ID}.desktop', directory / 'usr/share/applications' / f'{APP_ID}.desktop')
    copy(ROOT / 'gnome/data' / f'{APP_ID}.metainfo.xml', directory / 'usr/share/metainfo' / f'{APP_ID}.metainfo.xml')
    copy(ROOT / 'app/Assets/Bibbia.png', directory / 'usr/share/icons/hicolor/256x256/apps' / f'{APP_ID}.png')
    doc = directory / 'usr/share/doc/bibbia'
    for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
        copy(ROOT / name, doc / name)
    for name in ['LICENSE.bible-data', 'LICENSE.cei2008.txt', 'LICENSE.gutenberg.txt', 'cei2008-source.json', 'extended-source.json', 'martini-source.json', 'diodati-source.json']:
        copy(ROOT / 'core/data' / name, doc / name)
    for name in ['LICENSE.Adwaita', 'LICENSE.SourceSerif']:
        copy(ROOT / 'assets/fonts' / name, doc / name)
    directory.chmod(0o755)
    for path in directory.rglob('*'):
        if path.is_dir():
            path.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--skip-build', action='store_true')
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/bibbia')
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() not in ('x86_64', 'amd64'):
        raise SystemExit('This package recipe targets Linux x86-64.')
    for command in ['cargo', 'dpkg-deb', 'rpmbuild', 'rpmdb', 'desktop-file-validate', 'appstream-util', 'readelf']:
        if not shutil.which(command):
            raise SystemExit(f'Missing prerequisite: {command}')
    if not args.skip_build:
        run('cargo', 'build', '--release', '--locked', '-p', 'bibbia-gnome', cwd=ROOT)
    binary = args.binary.resolve()
    assert run(binary, '--version', capture_output=True, text=True).stdout.startswith(f'Bibbia {VERSION} ')
    dynamic = run('readelf', '-d', binary, capture_output=True, text=True).stdout
    assert not re.search(r'\((RUNPATH|RPATH)\)', dynamic), 'Do not ship build-machine library paths'
    symbols = run('readelf', '--version-info', binary, capture_output=True, text=True).stdout
    required = max(tuple(map(int, value.split('.'))) for value in re.findall(r'GLIBC_([0-9.]+)', symbols))
    assert required <= (2, 36), f'Build against the Debian 12 baseline (required GLIBC {required})'
    run('desktop-file-validate', ROOT / 'gnome/data' / f'{APP_ID}.desktop')
    run('appstream-util', 'validate-relax', '--nonet', ROOT / 'gnome/data' / f'{APP_ID}.metainfo.xml')
    output = ROOT / 'artifacts/linux'
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='bibbia-packaging-') as temporary:
        work = Path(temporary)
        stage = work / 'payload'
        payload(stage, binary)
        installed = sum(path.stat().st_size for path in stage.rglob('*') if path.is_file())
        control = work / 'deb/DEBIAN'
        shutil.copytree(stage, work / 'deb')
        control.mkdir()
        control.chmod(0o755)
        (control / 'control').write_text(f'''Package: bibbia
Version: {VERSION}-1
Section: education
Priority: optional
Architecture: amd64
Maintainer: Bibbia contributors
Homepage: https://github.com/Michele-Serafini/Bibbia
Installed-Size: {(installed + 1023) // 1024}
Depends: libc6 (>= 2.36), libgtk-4-1 (>= 4.8), libadwaita-1-0 (>= 1.2), libfontconfig1 (>= 2.13), hicolor-icon-theme
Description: Native GNOME offline Bible reader
 Catholic Italian CEI and Martini, Diodati, KJV, and an extended English
 collection with apocrypha. Phrase highlights, notes, bookmarks and search.
 Local data without accounts. CEI text is for educational noncommercial use;
 its separate terms and the font licenses are included in the package.
''', encoding='utf-8')
        (control / 'control').chmod(0o644)
        (control / 'md5sums').write_text(''.join(hashlib.md5(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(stage))+'\n' for p in sorted(stage.rglob('*')) if p.is_file()))
        (control / 'md5sums').chmod(0o644)
        run('dpkg-deb', '--root-owner-group', '-Zxz', '-z6', '--build', work / 'deb', output / f'bibbia_{VERSION}-1_amd64.deb')
        rpm = work / 'rpm'
        for directory in ['BUILD', 'BUILDROOT', 'RPMS', 'SOURCES', 'SPECS', 'SRPMS']:
            (rpm / directory).mkdir(parents=True)
        spec = rpm / 'SPECS/bibbia.spec'
        # Minimal Fedora images commonly install with tsflags=nodocs. License
        # notices are mandatory content, so preserve them as RPM LICENSE files.
        files = '\n'.join(f'%{"license " if path.name.startswith("LICENSE") or path.name == "THIRD_PARTY_NOTICES.md" else ""}%attr({"0755" if path == stage / "usr/bin/bibbia" else "0644"},root,root) /{path.relative_to(stage)}' for path in sorted(stage.rglob('*')) if path.is_file())
        spec.write_text(f'''Name: bibbia
Version: {VERSION}
Release: 1
Summary: Native GNOME offline Bible reader
License: MIT AND OFL-1.1 AND LicenseRef-CEI-Educational
URL: https://github.com/Michele-Serafini/Bibbia
BuildArch: x86_64
Prefix: /usr
Requires: gtk4 >= 4.8
Requires: libadwaita >= 1.2
Requires: fontconfig >= 2.13
Requires: hicolor-icon-theme
%define _binary_payload w6.xzdio
%define _build_id_links none
%description
Read the Bible offline with native GNOME controls. Phrase highlights,
notes, bookmarks and search stay on the device. Bible texts and fonts
have separate licenses. CEI is for educational noncommercial use.
%prep
%build
%install
mkdir -p "%{{buildroot}}"
cp -a {shlex.quote(str(stage))}/. "%{{buildroot}}"/
%files
{files}
''', encoding='utf-8')
        (work / 'rpm-tmp').mkdir()
        (work / 'rpm-db').mkdir()
        run('rpmdb', '--dbpath', work / 'rpm-db', '--initdb')
        run('rpmbuild', '-bb', '--define', f'_topdir {rpm}', '--define', f'_tmppath {work / "rpm-tmp"}', '--define', f'_dbpath {work / "rpm-db"}', spec)
        packages = list((rpm / 'RPMS').glob('**/*.rpm'))
        assert len(packages) == 1, packages
        copy(packages[0], output / f'bibbia-{VERSION}-1.x86_64.rpm')
    files = [output / f'bibbia_{VERSION}-1_amd64.deb', output / f'bibbia-{VERSION}-1.x86_64.rpm']
    for path in files:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        path.with_name(path.name + '.sha256').write_text(f'{digest}  {path.name}\n', encoding='ascii')
        print(f'{path.name}: {path.stat().st_size:,} bytes, SHA256 {digest}')
    print(f'Installed payload: {installed:,} bytes; GTK/libadwaita are shared system dependencies.')

if __name__ == '__main__':
    main()
