#!/usr/bin/env python3
"""Create deterministic mmap-friendly resources. No downloads or runtime Python."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct

ROOT = Path(__file__).resolve().parents[1]
EDITIONS = [
    ('martini', 'Martini 1781 (cattolica)', 'it', 'catholic'),
    ('cei2008', 'CEI 2008', 'it', 'catholic'),
    ('diodati', 'Diodati 1649', 'it', 'protestant'),
    ('kjv', 'KJV', 'en', 'protestant'),
    ('extended', 'Raccolta estesa (83 sezioni)', 'en', 'extended'),
]


def prepare(destination):
    corpora = destination / 'Corpora'
    notices = destination / 'Notices'
    corpora.mkdir(parents=True, exist_ok=True)
    notices.mkdir(parents=True, exist_ok=True)
    editions = []
    for file, name, language, canon in EDITIONS:
        raw = (ROOT / 'core/data' / f'{file}.json').read_bytes()
        if file == 'kjv':
            expected = 'f627b3c24a617013a4514ab24b6dd1bb305b09af237994c018f74cb9c7e71f41'
        else:
            meta = json.loads((ROOT / 'core/data' / f'{file}-source.json').read_bytes())
            expected = meta.get('converted_sha256', meta.get('sha256'))
        if hashlib.sha256(raw).hexdigest() != expected:
            raise ValueError(f'Corpus checksum mismatch: {file}')
        books = json.loads(raw)
        catalog, offset, lower_offset, text, lower = [], [0], [0], bytearray(), bytearray()
        for book in books:
            chapters = []
            for ci, verses in enumerate(book['chapters']):
                chapters.append(dict(first=len(offset) - 1, count=len(verses),
                                     labels=book.get('labels', [[]] * len(book['chapters']))[ci]))
                for verse in verses:
                    text.extend(verse.encode('utf-8'))
                    lower.extend(verse.lower().encode('utf-8'))
                    offset.append(len(text))
                    lower_offset.append(len(lower))
            catalog.append(dict(name=book['name'], abbrev=book['abbrev'], chapters=chapters))
        for suffix, data in [('text', text), ('lower', lower),
                             ('offsets', struct.pack(f'<{len(offset)}I', *offset)),
                             ('lower-offsets', struct.pack(f'<{len(lower_offset)}I', *lower_offset))]:
            (corpora / f'{file}-{suffix}.bin').write_bytes(data)
        (corpora / f'{file}-catalog.json').write_text(
            json.dumps(catalog, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
        editions.append(dict(id=name, file=file, language=language, canon=canon, books=len(books)))
    (corpora / 'editions.json').write_text(json.dumps(editions, ensure_ascii=False), encoding='utf-8')
    comparison = (ROOT / 'core/data/comparison.json').read_bytes()
    assert hashlib.sha256(comparison).hexdigest() == json.loads((ROOT / 'core/data/comparison-source.json').read_bytes())['converted_sha256']
    (corpora / 'comparison.json').write_bytes(comparison)
    for pattern in ('LICENSE*', '*-source.json'):
        for source in (ROOT / 'core/data').glob(pattern):
            shutil.copyfile(source, notices / source.name)
    for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
        shutil.copyfile(ROOT / name, notices / name)
    print(f'Prepared {len(editions)} verified editions in {destination}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path,
                        default=ROOT / 'macos/Sources/BibleKit/Resources')
    prepare(parser.parse_args().output)
