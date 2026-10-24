#!/usr/bin/env python3
"""Reproduce the CEI corpus from official pages (pip install beautifulsoup4 lxml).

Supply --cache with previously downloaded pages; --download fetches missing pages.
The manifest pins every input. The text has its own educational/noncommercial
terms, documented in core/data/LICENSE.cei2008.txt, and is NOT MIT licensed.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import subprocess

from bs4 import BeautifulSoup, SoupStrainer

root = Path(__file__).resolve().parents[1]
metadata_path = root / 'core/data/cei2008-source.json'
metadata = json.loads(metadata_path.read_text(encoding='utf-8'))
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cache', type=Path, required=True)
parser.add_argument('--download', action='store_true')
args = parser.parse_args()
args.cache.mkdir(parents=True, exist_ok=True)


def chapter(url):
    path = args.cache / (url.strip('/').replace('/', '_') + '.html')
    if not path.exists() and args.download:
        subprocess.run(['curl', '-fsSL', '--retry', '2', '--max-time', '35',
                        'https://www.bibbiaedu.it' + url, '-o', str(path)], check=True)
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    expected = metadata.get('chapter_sha256', {}).get(url)
    if expected and digest != expected:
        raise ValueError(f'Source checksum mismatch: {url}')
    soup = BeautifulSoup(raw, 'lxml', parse_only=SoupStrainer('div', class_='verses-container'))
    container = soup.select_one('.verses-container')
    assert container is not None, url
    rows = []
    parallel_esther = '/Est/' in url and 'TESTO EBRAICO' in container.get_text()
    for verse in container.select('span.verse'):
        speech = verse.select('.text-to-speech')
        assert speech, (url, 'verse without reading text')
        number = verse.select_one('.verse_number,button.show-note')
        assert number is not None, (url, str(verse)[:200])
        label = number.get_text('', strip=True)
        # The official Esther page presents BOTH Greek and Hebrew texts. Keep
        # both intact, with G/E labels to distinguish otherwise identical refs.
        if parallel_esther:
            heading = verse.find_previous(class_='verse_chapter_text').get_text(' ', strip=True)
            assert heading in ('TESTO GRECO', 'TESTO EBRAICO'), (url, heading)
            label = ('G' if heading == 'TESTO GRECO' else 'E') + label
        for tag in verse.select('.verse_number,button.show-note'):
            tag.decompose()
        for tag in verse.select('br'):
            tag.replace_with(' ')
        text = ' '.join(' '.join(t.get_text('', strip=False) for t in speech).split())
        assert text and label and label not in {n for n, _ in rows}, (url, label)
        rows.append((label, text))
    assert rows, url
    return url, rows, digest, container.select_one('.book-title').get_text(' ', strip=True)


urls = [f'{book["path"]}{n}/' for book in metadata['books'] for n in range(1, book['chapters'] + 1)]
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    pages = {u: (rows, digest, name) for u, rows, digest, name in pool.map(chapter, urls)}
result = []
for book in metadata['books']:
    keys = [f'{book["path"]}{n}/' for n in range(1, book['chapters'] + 1)]
    result.append(dict(name=pages[keys[0]][2], abbrev=book['path'].strip('/').rsplit('/', 1)[1],
                       chapters=[[t for _, t in pages[k][0]] for k in keys],
                       labels=[[n for n, _ in pages[k][0]] for k in keys]))
assert len(result) == 73 and len(urls) == 1328
assert result[0]['chapters'][0][0] == 'In principio Dio creò il cielo e la terra.'
encoded = (json.dumps(result, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
(root / 'core/data/cei2008.json').write_bytes(encoded)
metadata.update(name='CEI 2008', language='it', source='https://www.bibbiaedu.it/CEI2008/',
                license_evidence='https://www.bibbiaedu.it/site/twig/templates/v3/html/notelegali/',
                copyright='© 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena',
                rights='Educational and formative noncommercial uses only; see LICENSE.cei2008.txt. Not public domain or MIT.',
                retrieved='2026-10-24', sha256=hashlib.sha256(encoded).hexdigest(),
                chapter_sha256={u: pages[u][1] for u in urls},
                conversion='Bible text only, without commentary or number buttons. Original text and verse labels preserved; entities decoded and layout whitespace normalized. Esther: G=Greek, E=Hebrew, both official texts retained. Lettered and combined verses remain distinct records.')
metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'CEI 2008: 73 books, {len(urls)} chapters, {sum(len(c) for b in result for c in b["chapters"])} records; SHA256 {metadata["sha256"]}')
