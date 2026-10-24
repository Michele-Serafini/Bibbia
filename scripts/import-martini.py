#!/usr/bin/env python3
"""Reproduce the bundled public-domain Martini corpus from a pinned LaParola commit.

Python standard library and curl only. Existing downloads in --cache are verified
against their Git blob hashes before reuse. This is a development tool; the app
never downloads Bible texts at runtime.
"""
import argparse
import concurrent.futures
import hashlib
import json
from html.parser import HTMLParser
from pathlib import Path
import re
import subprocess
import tempfile

COMMIT = '207f6a574ffa5913790e27962f5e9e180ce1b47c'
# Git tree for the martini directory at COMMIT; avoid truncating the enormous
# recursive tree of the full LaParola repository.
TREE = 'deba63ae1f1851df83d03fa2cded89d876fb2721'
PREFIX = 'Sito/Pagine/app/content/texts/martini/'
BASE = f'https://raw.githubusercontent.com/LPN6/LaParola/{COMMIT}/{PREFIX}'


class Verses(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.depth = 0
        self.number = None
        self.text = []
        self.verses = {}

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'span':
            if self.depth:
                self.depth += 1
            elif ('v' in attrs.get('class', '').split()
                  and re.fullmatch(r'.+_\d+', attrs.get('data-id', ''))):
                self.number = int(attrs['data-id'].rsplit('_', 1)[1])
                self.depth = 1
                self.text = []

    def handle_data(self, text):
        if self.depth:
            self.text.append(text)

    def handle_endtag(self, tag):
        if tag == 'span' and self.depth:
            self.depth -= 1
            if not self.depth:
                assert self.number not in self.verses, 'Duplicate verse'
                self.verses[self.number] = ' '.join(''.join(self.text).split())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path,
                        default=Path(tempfile.gettempdir()) / f'bibbia-martini-{COMMIT}')
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)

    def download(name, url):
        path = args.cache / name
        if not path.exists():
            subprocess.run(['curl', '-fsSL', '--retry', '2', '--max-time', '30',
                            url, '-o', str(path)], check=True)
        return path.read_bytes()

    tree = json.loads(download('martini-git-tree.json',
        f'https://api.github.com/repos/LPN6/LaParola/git/trees/{TREE}'))
    assert not tree['truncated'], 'Incomplete source tree'
    blobs = {item['path']: item['sha'] for item in tree['tree']}

    def source(name):
        data = download(name, BASE + name)
        blob = b'blob ' + str(len(data)).encode() + b'\0' + data
        assert hashlib.sha1(blob).hexdigest() == blobs[name], f'Hash mismatch: {name}'
        return data.decode('utf-8-sig')

    assert '<dd>Dominio pubblico</dd>' in source('about.html'), 'Missing rights evidence'
    info = json.loads(source('info.json'))

    def chapter(section):
        verses = Verses()
        verses.feed(source(section + '.html'))
        numbers = sorted(n for n in verses.verses if n > 0)
        assert numbers and numbers == list(range(1, numbers[-1] + 1)), section
        return section, [verses.verses[n] for n in numbers]

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        chapters = dict(pool.map(chapter, info['sections']))
    books = []
    for abbrev, name in zip(info['divisions'], info['divisionNames'], strict=True):
        keys = [s for s in info['sections'] if re.fullmatch(re.escape(abbrev) + r'\d+', s)]
        keys.sort(key=lambda s: int(s[len(abbrev):]))
        assert [int(k[len(abbrev):]) for k in keys] == list(range(1, len(keys) + 1)), abbrev
        books.append(dict(name=name, abbrev=abbrev, chapters=[chapters[k] for k in keys]))
    assert len(books) == 73 and len(chapters) == 1334
    data = (json.dumps(books, ensure_ascii=False, separators=(',', ':')) + '\n').encode('utf-8')
    root = Path(__file__).resolve().parents[1]
    metadata = json.loads((root / 'core/data/martini-source.json').read_text(encoding='utf-8'))
    assert hashlib.sha256(data).hexdigest() == metadata['sha256'], 'Converted corpus differs'
    (root / 'core/data/martini.json').write_bytes(data)
    print(f'Verified {len(books)} books, {len(chapters)} chapters; SHA-256 {metadata["sha256"]}')


if __name__ == '__main__':
    main()
