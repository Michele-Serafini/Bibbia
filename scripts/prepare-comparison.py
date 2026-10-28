#!/usr/bin/env python3
"""Build shared book identities and Psalm reference groups from pinned TVTMS data.

The app uses the generated JSON offline. No Bible text is modified. Only the
Psalm tables are extracted; unsupported versification differences stay explicit.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
OT = 'Gen Exod Lev Num Deut Josh Judg Ruth 1Sam 2Sam 1Kgs 2Kgs 1Chr 2Chr Ezra Neh Esth Job Ps Prov Eccl Song Isa Jer Lam Ezek Dan Hos Joel Amos Obad Jonah Mic Nah Hab Zeph Hag Zech Mal'.split()
NT = 'Matt Mark Luke John Acts Rom 1Cor 2Cor Gal Eph Phil Col 1Thess 2Thess 1Tim 2Tim Titus Phlm Heb Jas 1Pet 2Pet 1John 2John 3John Jude Rev'.split()
CATHOLIC = OT[:16] + ['Tob', 'Jdt', 'Esth', '1Macc', '2Macc', 'Job', 'Ps', 'Prov', 'Eccl', 'Song', 'Wis', 'Sir', 'Isa', 'Jer', 'Lam', 'Bar'] + OT[25:] + NT
EXTENDED = OT + 'Tob Jdt EsthGr Wis Sir Bar 1Macc 2Macc 1Esd PrMan Ps151 3Macc 2Esd 4Macc DanGr'.split() + NT + ['1En', 'Jub']


def normalize(name):
    return ''.join(c for c in name.lower() if c.isalnum())


def references(value):
    m = re.fullmatch(r'Psa\.(\d+):(Title|\d+)(?:-(\d+))?', value.strip())
    if not m:
        return []
    chapter, first, last = m.groups()
    if first == 'Title':
        return [f'{chapter}:Title']
    return [f'{chapter}:{v}' for v in range(int(first), int(last or first)+1)]


def prepare(source):
    raw = source.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == '63058e0f20201af4bdaa7d830da5be8f493455d947c5f147d84840b33db9ddf8', "TVTMS source checksum mismatch"
    aliases, books = {}, {}
    for file in ['cei2008', 'martini', 'diodati', 'kjv', 'extended']:
        data = json.loads((ROOT / f'core/data/{file}.json').read_bytes())
        codes = CATHOLIC if file in ['cei2008', 'martini'] else EXTENDED if file == 'extended' else OT+NT
        assert len(codes) == len(data), file
        books[file] = (data, codes)
        for book, code in zip(data, codes, strict=True):
            key = normalize(book['name'])
            assert key not in aliases or aliases[key] == code, key
            aliases[key] = code

    # Shared groups preserve splits/merges rather than duplicating or dropping
    # verses. All original references remain visible in the native UI.
    parent = {}
    def find(key):
        parent.setdefault(key, key)
        if parent[key] != key:
            parent[key] = find(parent[key])
        return parent[key]
    def union(a, b):
        a, b = find(a), find(b)
        if a != b:
            parent[max(a, b)] = min(a, b)
    mappings = {profile: {} for profile in ['English KJV', 'Hebrew', 'Latin']}
    active, headers = False, []
    for line in raw.decode('utf-8-sig').splitlines():
        if line.startswith('#DataEnd(Condensed)'):
            break
        cells = line.split('\t')
        if line.startswith('$'):
            active = line.startswith('$Psa.')
        if not active:
            continue
        if cells[0] == 'BIBLES':
            headers = [c.strip() for c in cells[1:]]
        elif cells[0] in ['OneToOne', 'SubdividedVerse', 'MergedPrevVerse', 'MergedFollVerse'] and headers:
            columns = {h: references(v) for h, v in zip(headers, cells[1:])}
            english = columns.get('English KJV', [])
            if not english:
                continue
            for profile in mappings:
                local = columns.get(profile, [])
                for i, ref in enumerate(local):
                    targets = [english[i]] if len(local) == len(english) else english
                    key = targets[0]
                    for target in targets[1:]:
                        union(key, target)
                    if ref in mappings[profile]:
                        union(key, mappings[profile][ref])
                    mappings[profile][ref] = key

    psalms = {}
    for file, (data, codes) in books.items():
        book = data[codes.index('Ps')]
        profile = 'Hebrew' if file == 'cei2008' else 'Latin' if file == 'martini' else 'English KJV'
        mapping = mappings[profile]
        chapters = []
        unsupported = []
        for ci, verses in enumerate(book['chapters'], 1):
            numeric = {int(ref.split(':')[1]): key for ref, key in mapping.items()
                       if ref.startswith(f'{ci}:') and ref.split(':')[1].isdigit()}
            shift = 0
            if numeric:
                title_count = sum('Title' in find(key) for key in numeric.values())
                if len(verses) == max(numeric)-title_count:
                    shift = title_count
                elif len(verses) != max(numeric):
                    unsupported.append(ci)
                    chapters.append([f'unmapped:{file}:{ci}:{vi}' for vi in range(1,len(verses)+1)])
                    continue
            chapters.append([f'Ps:{find(numeric.get(vi+shift, f"{ci}:{vi+shift}"))}' for vi in range(1,len(verses)+1)])
        psalms[file] = chapters
        print(file, 'unverified Psalm chapters:', unsupported)
    result = dict(aliases=aliases, psalms=psalms)
    output = ROOT / 'core/data/comparison.json'
    output.write_text(json.dumps(result, ensure_ascii=False, separators=(',', ':'))+'\n')
    metadata = dict(source='https://github.com/STEPBible/STEPBible-Data/tree/master/Versification',
                    sha256=hashlib.sha256(raw).hexdigest(), license='CC BY 4.0',
                    converted_sha256=hashlib.sha256(output.read_bytes()).hexdigest(),
                    attribution='STEPBible.org, Tyndale House Cambridge',
                    conversion='Psalm condensed tables only; shared reference groups for splits/merges; omitted numbered titles adapted to the bundled corpus counts; unverified chapters marked unmapped. Book aliases and Joel/Malachi rules are project data.')
    (ROOT / 'core/data/comparison-source.json').write_text(json.dumps(metadata,indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path,help='Downloaded TVTMS source file')
    prepare(parser.parse_args().source)
