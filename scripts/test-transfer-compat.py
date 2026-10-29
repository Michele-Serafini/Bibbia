#!/usr/bin/env python3
"""Exercise Rust -> Swift -> Rust backups. Use with the optional Swift fixture test."""
import argparse, ctypes, json
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('phase',choices=['prepare','verify'])
parser.add_argument('directory',type=Path)
parser.add_argument('--library',type=Path,default=Path('target/release/libbible_core.so'))
a=parser.parse_args();root=a.directory.resolve()
assert root.name.startswith('bibbia-transfer-')
root.mkdir(parents=True,exist_ok=True);(root/'.bibbia-transfer-fixture').write_text('owned cross-engine fixture')
lib=ctypes.CDLL(str(a.library.resolve()));lib.bible_request.argtypes=[ctypes.c_char_p];lib.bible_request.restype=ctypes.c_void_p;lib.bible_free.argtypes=[ctypes.c_void_p]
def call(**req):
 p=lib.bible_request(json.dumps(req,ensure_ascii=False).encode())
 try:result=json.loads(ctypes.string_at(p))
 finally:lib.bible_free(p)
 assert result['ok'],result
 return result['data']
if a.phase=='prepare':
 call(op='init',directory=str(root/'rust-source'))
 source=root/'edition.json';source.write_text(json.dumps([dict(name='Libro Ω 🌅',abbrev='LU',chapters=[['Luce 🌅 e pace']])],ensure_ascii=False))
 call(op='import',id='Traduzione personale',path=str(source))
 position=dict(edition='Traduzione personale',book=0,chapter=0,verse=0)
 annotation=call(op='note',**position,text='Perché Ω 🌅',color='blue',style='sticky');annotation['bookmark']=True
 call(op='annotate',**position,annotation=annotation)
 call(op='mark',**position,start=5,length=2,action='underline')
 call(op='organize',**position,folder='Studio',tags=['speranza'])
 plan=call(op='reading',action='create',edition=position['edition'],title='Lettura',start='2026-10-29',daily=1,book=0)
 call(op='reading',action='complete',id=plan,index=0,complete=True)
 call(op='settings',key='position',value=position)
 call(op='transfer',action='backup',path=str(root/'rust.bibbia-backup'))
 call(op='close')
 print('Rust backup ready for the Swift fixture.')
else:
 call(op='init',directory=str(root/'rust-destination'))
 call(op='transfer',action='restore',path=str(root/'swift.bibbia-backup'))
 call(op='close');call(op='init',directory=str(root/'rust-destination'))
 row=call(op='saved',edition='Traduzione personale')[0]
 assert row['annotation']['note']=='Aggiornata Swift 🌅'
 assert row['annotation']['noteColor']=='green' and row['annotation']['noteStyle']=='sticky'
 assert row['annotation']['bookmark'] and row['annotation']['ranges'][0]['start']==5 and row['annotation']['ranges'][0]['length']==2
 assert row['organization']==dict(folder='Studio',tags=['speranza'])
 assert call(op='reading',action='detail',id=1,**{'from':'2026-10-29'},selected='2026-10-29')['completed']==1
 assert call(op='bootstrap')['settings']['position']['edition']=='Traduzione personale'
 call(op='close');print('Rust -> Swift -> Rust: imported corpus, Unicode, UTF-16 marks, colors, organization, position and reading progress preserved.')
