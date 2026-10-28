#!/usr/bin/env python3
"""Measure two complete native apps on the same machine with isolated profiles.

The baseline is v0.9.0 plus the same explicit fixture hooks. No forced garbage
collection, allocator purge or working-set trimming. Display-server memory is
excluded. Keep private memory, RSS and CPU separate; do not compare OS totals.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import statistics
import subprocess
import tempfile
import time

PHASES = ['initial','long_chapter','comparison','returned','wide','narrow','cycled','animation','settled']


def memory(pid):
    if sys_platform == 'Linux':
        data = {}
        for line in Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines():
            if ':' in line:
                key,value=line.split(':',1);data[key]=int(value.split()[0])
        fields=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
        return dict(rss_mib=data['Rss']/1024,pss_mib=data['Pss']/1024,
                    private_mib=(data['Private_Clean']+data['Private_Dirty'])/1024,
                    cpu_seconds=(int(fields[11])+int(fields[12]))/os.sysconf('SC_CLK_TCK'))
    if sys_platform == 'Darwin':
        values=subprocess.check_output(['ps','-p',str(pid),'-o','rss=,time='],text=True).split()
        parts=values[1].replace('-',':').split(':')
        return dict(rss_mib=int(values[0])/1024,cpu_seconds=sum(float(v)*f for v,f in zip(reversed(parts),[1,60,3600,86400])))
    from ctypes import wintypes as W
    class Counters(ctypes.Structure):
        _fields_=[('cb',W.DWORD),('faults',W.DWORD)]+[(n,ctypes.c_size_t) for n in ['peak_rss','rss','peak_pool_paged','pool_paged','peak_pool_nonpaged','pool_nonpaged','pagefile','peak_pagefile','private']]
    class FileTime(ctypes.Structure):
        _fields_=[('low',W.DWORD),('high',W.DWORD)]
    kernel=ctypes.WinDLL('kernel32',use_last_error=True);psapi=ctypes.WinDLL('psapi',use_last_error=True)
    kernel.OpenProcess.argtypes=[W.DWORD,W.BOOL,W.DWORD];kernel.OpenProcess.restype=W.HANDLE
    kernel.CloseHandle.argtypes=[W.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes=[W.HANDLE,ctypes.POINTER(Counters),W.DWORD]
    kernel.GetProcessTimes.argtypes=[W.HANDLE]+[ctypes.POINTER(FileTime)]*4
    handle=kernel.OpenProcess(0x410,False,pid)
    if not handle:raise ctypes.WinError(ctypes.get_last_error())
    try:
        info=Counters();info.cb=ctypes.sizeof(info)
        if not psapi.GetProcessMemoryInfo(handle,ctypes.byref(info),info.cb):raise ctypes.WinError(ctypes.get_last_error())
        created,exited,kernel_time,user_time=FileTime(),FileTime(),FileTime(),FileTime()
        if not kernel.GetProcessTimes(handle,ctypes.byref(created),ctypes.byref(exited),ctypes.byref(kernel_time),ctypes.byref(user_time)):raise ctypes.WinError(ctypes.get_last_error())
        seconds=sum((v.high<<32|v.low) for v in [kernel_time,user_time])/10_000_000
        return dict(rss_mib=info.rss/1048576,private_mib=info.private/1048576,cpu_seconds=seconds)
    finally:kernel.CloseHandle(handle)


def advance(pid):
    if sys_platform != 'Windows':os.kill(pid,signal.SIGUSR1);return
    from ctypes import wintypes as W
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.OpenEventW.argtypes=[W.DWORD,W.BOOL,W.LPCWSTR];kernel.OpenEventW.restype=W.HANDLE
    kernel.SetEvent.argtypes=[W.HANDLE];kernel.CloseHandle.argtypes=[W.HANDLE]
    handle=kernel.OpenEventW(2,False,f'Local\\Bibbia.Performance.{pid}')
    if not handle:raise ctypes.WinError(ctypes.get_last_error())
    try:
        if not kernel.SetEvent(handle):raise ctypes.WinError(ctypes.get_last_error())
    finally:kernel.CloseHandle(handle)


def exercise(binary,label,index,logs):
    with tempfile.TemporaryDirectory(prefix='bibbia-perf-') as temporary:
        profile=Path(temporary);(profile/'.bibbia-perf-fixture').write_text('owned disposable performance fixture\n')
        env=dict(os.environ,BIBBIA_PERFORMANCE_PROFILE=str(profile),BIBBIA_UI_TEST_MOTION='1')
        with (logs/f'{label}-{index}.log').open('w') as log:
            started=time.perf_counter();process=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=log)
            phases={}
            try:
                for phase in PHASES:
                    marker=profile/f'{phase}.json';deadline=time.monotonic()+90
                    while not marker.exists():
                        if process.poll() is not None or time.monotonic()>deadline:
                            details=(profile/'error.txt').read_text() if (profile/'error.txt').exists() else (logs/f'{label}-{index}.log').read_text(errors='replace')[-4000:]
                            raise RuntimeError(f'{label}/{phase} did not complete: {details}')
                        time.sleep(0.01)
                    native=json.loads(marker.read_text(encoding='utf-8-sig'))
                    if phase=='initial':native['startup_ready_ms']=(time.perf_counter()-started)*1000
                    time.sleep(0.04 if phase=='animation' else 0.3)
                    first=memory(process.pid);before=time.perf_counter();samples=[first]
                    for _ in range(3):time.sleep(0.04 if phase=='animation' else 0.12);samples.append(memory(process.pid))
                    elapsed=time.perf_counter()-before
                    measured={key:statistics.median(s[key] for s in samples) for key in first if key!='cpu_seconds'}
                    measured['cpu_percent']=(samples[-1]['cpu_seconds']-first['cpu_seconds'])/elapsed*100
                    phases[phase]={**native,**measured}
                    advance(process.pid)
                code=process.wait(timeout=20)
                if code:raise RuntimeError(f'{label} exited with {code}')
                return phases
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:process.wait(timeout=5)
                    except subprocess.TimeoutExpired:process.kill();process.wait()


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline',type=Path,required=True)
    parser.add_argument('--candidate',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--samples',type=int,default=3)
    args=parser.parse_args();sys_platform=platform.system()
    logs=args.output.parent/'performance-logs';logs.mkdir(parents=True,exist_ok=True)
    rows={'baseline':[],'candidate':[]}
    for index in range(args.samples):
        for label in (['baseline','candidate'] if index%2==0 else ['candidate','baseline']):
            print(f'Native performance: {label}, sample {index+1}',flush=True)
            rows[label].append(exercise(getattr(args,label).resolve(),label,index,logs))
    medians={label:{phase:{key:round(statistics.median(row[phase][key] for row in samples),3) for key in samples[0][phase]} for phase in PHASES} for label,samples in rows.items()}
    result=dict(platform=platform.platform(),architecture=platform.machine(),
        method='Same runner, native release apps, fresh marked profiles, alternating order, 3 samples. Same fixture hooks in v0.9.0 baseline and candidate. 1000x720 narrow / 1500x900 wide, CEI Psalm 119, KJV comparison, 8 navigation/comparison cycles. First-ready includes a 120ms one-shot layout settle; ordinary idle benchmarks are separate. No forced GC, heap purge or working-set trimming. CPU is one core; excludes display server and GPU allocations. RSS/private/PSS are different metrics; do not compare totals between operating systems.',
        binaries={label:dict(path=str(getattr(args,label)),sha256=hashlib.file_digest(getattr(args,label).open('rb'),'sha256').hexdigest()) for label in rows},medians=medians,samples=rows)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(medians,indent=2))
