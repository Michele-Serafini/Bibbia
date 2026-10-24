"""Linux native-core microbenchmark; includes the Python harness, not WinUI RAM.

Build release first; pass an optional .so path to compare the same fixture.
Seven fresh processes, 200 Genesis chapters, 40 absent-query full scans.
"""
import ctypes,json,time,tempfile,resource,statistics,subprocess,sys
from pathlib import Path
if len(sys.argv)>1 and sys.argv[1]=='--worker':
 lib=ctypes.CDLL(sys.argv[2]);lib.bible_request.argtypes=[ctypes.c_char_p];lib.bible_request.restype=ctypes.c_void_p;lib.bible_free.argtypes=[ctypes.c_void_p]
 def call(**req):
  p=lib.bible_request(json.dumps(req).encode())
  try:r=json.loads(ctypes.string_at(p))
  finally:lib.bible_free(p)
  assert r['ok'],r
  return r['data']
 def timed(fn,n=1):
  t=time.perf_counter()
  for _ in range(n):fn()
  return (time.perf_counter()-t)*1000/n
 with tempfile.TemporaryDirectory() as d:
  r={};r['init_ms']=timed(lambda:call(op='init',directory=d));e='Martini 1781 (cattolica)'
  r['chapter_ms']=timed(lambda:call(op='chapter',edition=e,book=0,chapter=0),200)
  r['search_ms']=timed(lambda:call(op='search',edition=e,query='zzxyzznonexistent'),40)
  r['rss_mib']=int(Path('/proc/self/status').read_text().split('VmRSS:')[1].split()[0])/1024
  r['peak_mib']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024
  call(op='close');print(json.dumps(r))
else:
 library=str(Path(sys.argv[1] if len(sys.argv)>1 else Path(__file__).resolve().parents[1]/'target/release/libbible_core.so').resolve())
 rows=[json.loads(subprocess.check_output([sys.executable,__file__,'--worker',library])) for _ in range(7)]
 result={k:round(statistics.median(x[k] for x in rows),3) for k in rows[0]}
 print(json.dumps({'median':result,'samples':rows},indent=2))
