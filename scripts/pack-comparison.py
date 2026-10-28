#!/usr/bin/env python3
"""Pack the verified reference groups without changing any correspondence."""
import hashlib
import argparse
import json
from pathlib import Path
import struct

ROOT=Path(__file__).resolve().parents[1]/'core/data'
source=(ROOT/'comparison.json').read_bytes()
metadata=json.loads((ROOT/'comparison-source.json').read_bytes())
assert hashlib.sha256(source).hexdigest()==metadata['converted_sha256']
data=json.loads(source)
assert data['psalms']['diodati']==data['psalms']['kjv']==data['psalms']['extended']
values=[];offsets=[]
for profile in ['cei2008','martini','kjv']:
    chapters=data['psalms'][profile];assert len(chapters)==150
    for chapter in chapters:
        offsets.append(len(values))
        for key in chapter:
            if key.startswith('unmapped:'):values.append(0);continue
            _,c,v=key.split(':');values.append(int(c)*256+(0 if v=='Title' else int(v)))
    offsets.append(len(values))
packed=b'BPC1'+struct.pack(f'<{len(offsets)}I',*offsets)+struct.pack(f'<{len(values)}H',*values)
aliases=(json.dumps({'aliases':data['aliases']},ensure_ascii=False,separators=(',',':'))+'\n').encode('utf-8')
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true',help='Verify committed runtime tables without rewriting them')
check=parser.parse_args().check
for name,content in [('comparison-psalms.bin',packed),('comparison-books.json',aliases)]:
    if check:
        assert (ROOT/name).read_bytes()==content,f'{name}: regenerate with scripts/pack-comparison.py'
    else:
        (ROOT/name).write_bytes(content)
print(f'Comparison runtime index: {len(source):,} → {len(packed)+len(aliases):,} bytes; all verified reference groups preserved')
