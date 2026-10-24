"""Check pinned corpus bytes, structure, counts, and source verse labels."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1] / 'core/data'
for name, count, chapters in [('kjv', 66, 1189), ('diodati', 66, 1189),
                              ('martini', 73, 1334), ('cei2008', 73, 1328),
                              ('extended', 83, 1560)]:
    raw = (root / f'{name}.json').read_bytes()
    if name == 'kjv':
        expected = 'f627b3c24a617013a4514ab24b6dd1bb305b09af237994c018f74cb9c7e71f41'
    else:
        source = json.loads((root / f'{name}-source.json').read_text(encoding='utf-8'))
        expected = source.get('converted_sha256', source.get('sha256'))
    assert hashlib.sha256(raw).hexdigest() == expected, name
    books = json.loads(raw)
    assert len(books) == count and sum(len(b['chapters']) for b in books) == chapters, name
    assert len({b['abbrev'] for b in books}) == count, name
    for book in books:
        labels = book.get('labels')
        assert book['name'] and book['abbrev'] and book['chapters']
        if labels is not None:
            assert len(labels) == len(book['chapters'])
        for n, verses in enumerate(book['chapters']):
            assert verses and all(isinstance(t, str) and t.strip() for t in verses), (name, book['name'], n)
            if labels is not None:
                assert len(labels[n]) == len(verses) == len(set(labels[n]))
                assert all(isinstance(v, str) and v.strip() for v in labels[n])
    print(f'{name}: checksum, {count} books/sections, {chapters} chapters and labels verified')
