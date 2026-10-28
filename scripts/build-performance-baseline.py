#!/usr/bin/env python3
"""Build v0.9.0 + fixture hooks beside the candidate on the same native runner."""
from pathlib import Path
import argparse
import os
import platform
import re
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[1]
def run(*args,**kw):
    return subprocess.run([str(a) for a in args],check=True,**kw)

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output',type=Path,default=ROOT/'artifacts/performance-baseline')
args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
revision=(ROOT/'scripts/performance-baseline.txt').read_text().strip()
assert re.fullmatch(r'[0-9a-f]{40}',revision),'Use the full reviewed baseline commit'
source=out/'source'
if not source.exists():
    run('git','fetch','--no-tags','--depth=1','origin',revision,cwd=ROOT)
    run('git','worktree','add','--detach',source,revision,cwd=ROOT)
assert subprocess.check_output(['git','-c',f'safe.directory={source}','rev-parse','HEAD'],cwd=source,text=True).strip()==revision
native=out/'native';native.mkdir(exist_ok=True)
kind=platform.system()
if kind=='Windows':
    run('pwsh','-NoProfile','-File',ROOT/'scripts/build-performance-baseline.ps1','-SourceDirectory',source,'-Destination',native)
elif kind=='Darwin':
    run('python3',source/'scripts/prepare-macos.py')
    command=['swift','build','--package-path',source/'macos','--scratch-path',out/'swift-build','-c','release','--product','Bibbia','-Xswiftc','-Osize']
    run(*command)
    binary_dir=Path(subprocess.check_output([str(x) for x in command+['--show-bin-path']],text=True).strip())
    contents=native/'Bibbia.app/Contents';(contents/'MacOS').mkdir(parents=True,exist_ok=True);(contents/'Resources').mkdir(exist_ok=True)
    shutil.copy2(binary_dir/'Bibbia',contents/'MacOS/Bibbia')
    shutil.copy2(source/'macos/Info.plist',contents/'Info.plist')
    for bundle in binary_dir.glob('*BibleKit.bundle'):
        shutil.copytree(bundle,contents/'Resources'/bundle.name,dirs_exist_ok=True)
    run('codesign','--force','--deep','--sign','-','--options','runtime','--timestamp=none',native/'Bibbia.app')
else:
    target=ROOT/'target/performance-baseline'
    env=dict(os.environ,CARGO_TARGET_DIR=str(target))
    run('cargo','build','--release','--locked','-p','bibbia-gnome',cwd=source,env=env)
    shutil.copy2(target/'release/bibbia',native/'bibbia')
print('Native baseline source:',revision)
