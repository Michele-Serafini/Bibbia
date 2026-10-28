#!/usr/bin/env python3
"""Build a native universal Swift .app, drag-to-Applications DMG and ZIP."""
import hashlib
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def verify_dmg(image):
    # DiskImages can briefly retain the image after `create` exits on CI.
    # Retry only that transient error; malformed images must still fail.
    for attempt in range(4):
        result = subprocess.run(['hdiutil', 'verify', str(image)], text=True, capture_output=True)
        print(result.stdout, end='')
        print(result.stderr, end='', file=sys.stderr)
        if result.returncode == 0:
            return
        if attempt == 3 or 'Resource temporarily unavailable' not in result.stderr:
            result.check_returncode()
        time.sleep(attempt + 1)


def package():
    run(sys.executable, ROOT / 'scripts/check-versions.py')
    if sys.platform != 'darwin':
        raise SystemExit('This script requires macOS with Xcode Command Line Tools.')
    version = plistlib.loads((ROOT / 'macos/Info.plist').read_bytes())['CFBundleShortVersionString']
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise ValueError('Invalid bundle version')
    run(sys.executable, ROOT / 'scripts/prepare-macos.py')
    output = ROOT / 'artifacts/macos'
    staging = ROOT / 'artifacts/macos-staging'
    if staging.exists():
        shutil.rmtree(staging)
    output.mkdir(parents=True, exist_ok=True)
    contents = staging / 'Bibbia.app/Contents'
    executable = contents / 'MacOS/Bibbia'
    resources = contents / 'Resources'
    executable.parent.mkdir(parents=True)
    resources.mkdir()
    binaries = []
    for arch in ['arm64', 'x86_64']:
        build = ROOT / f'artifacts/macos-build-{arch}'
        command = ['swift', 'build', '--package-path', ROOT / 'macos', '--scratch-path', build,
                   '-c', 'release', '--triple', f'{arch}-apple-macosx13.0',
                   '-Xswiftc', '-Osize', '--product', 'Bibbia']
        run(*command)
        bin_dir = Path(subprocess.check_output(
            command + ['--show-bin-path'], text=True).strip())
        binaries.append(bin_dir / 'Bibbia')
        bundles = list(bin_dir.glob('*BibleKit.bundle'))
        if len(bundles) != 1:
            raise ValueError(f'Expected exactly one resource bundle in {bin_dir}')
        if arch == 'arm64':
            shutil.copytree(bundles[0], resources / bundles[0].name)
    run('lipo', '-create', *binaries, '-output', executable)
    run('lipo', executable, '-verify_arch', 'arm64', 'x86_64')
    run('strip', '-x', executable)
    shutil.copyfile(ROOT / 'macos/Info.plist', contents / 'Info.plist')
    iconset = ROOT / 'artifacts/Bibbia.iconset'
    iconset.mkdir(exist_ok=True)
    original = ROOT / 'artifacts/macos-icon.png'
    run('swift', ROOT / 'scripts/macos-icon.swift', original)
    for size in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            suffix = '@2x' if scale == 2 else ''
            run('sips', '-z', size * scale, size * scale, original,
                '--out', iconset / f'icon_{size}x{size}{suffix}.png', stdout=subprocess.DEVNULL)
    run('iconutil', '-c', 'icns', iconset, '-o', resources / 'Bibbia.icns')
    run('plutil', '-lint', contents / 'Info.plist')
    # Ad-hoc signing verifies integrity locally. Apple Developer ID signing and
    # notarization need the owner's credentials and are not claimed by this build.
    run('codesign', '--force', '--deep', '--sign', '-', '--options', 'runtime', '--timestamp=none', staging / 'Bibbia.app')
    run('codesign', '--verify', '--deep', '--strict', '--verbose=2', staging / 'Bibbia.app')
    (staging / 'Applications').symlink_to('/Applications', target_is_directory=True)
    shutil.copyfile(ROOT / 'macos/INSTALL.txt', staging / 'LEGGIMI.txt')
    dmg = output / f'Bibbia-{version}-universal.dmg'
    archive = output / f'Bibbia-{version}-universal.zip'
    run('hdiutil', 'create', '-volname', f'Bibbia {version}', '-srcfolder', staging,
        '-ov', '-format', 'UDZO', dmg)
    verify_dmg(dmg)
    run('ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', staging / 'Bibbia.app', archive)
    for artifact in [dmg, archive]:
        digest = hashlib.file_digest(artifact.open('rb'), 'sha256').hexdigest()
        artifact.with_name(artifact.name + '.sha256').write_text(f'{digest}  {artifact.name}\n')
        print(f'{artifact.name}: {artifact.stat().st_size:,} bytes')


if __name__ == '__main__':
    package()
