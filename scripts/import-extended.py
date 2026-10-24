"""Reproduce the extended public-domain collection (pip install beautifulsoup4).

Pass official WEB ZIPs, the Gutenberg Enoch HTML, and 50 cached Sefaria chapters. Source verse labels
(including combined verses) are preserved, rather than inventing missing verses.
"""
import collections
import hashlib
import json
from pathlib import Path
import re
import argparse
from bs4 import BeautifulSoup
import xml.etree.ElementTree as ET
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('vpl', type=Path)
parser.add_argument('usfm', type=Path)
parser.add_argument('--enoch', type=Path, required=True)
parser.add_argument('--jubilees-cache', type=Path, required=True)
args = parser.parse_args()
vpl, usfm = args.vpl, args.usfm
source, headers = zipfile.ZipFile(vpl), zipfile.ZipFile(usfm)
names = {}
for name in headers.namelist():
    if name.endswith('.usfm'):
        text = headers.read(name).decode('utf-8-sig')
        code, title = re.search(r'\\id (\w+)', text), re.search(r'\\h (.*)', text)
        if code and title:
            names[code[1]] = title[1].strip()
books = collections.OrderedDict()
for verse in ET.fromstring(source.read('eng-web_vpl.xml')):
    if verse.tag != 'v':
        continue
    code, chapter, label = verse.attrib['b'], int(verse.attrib['c']), verse.attrib['v']
    text = ''.join(verse.itertext()).strip()
    if text:
        books.setdefault(code, collections.OrderedDict()).setdefault(chapter, []).append((label, text))
result = []
for code, chapters in books.items():
    assert list(chapters) == list(range(1, len(chapters) + 1)), code
    assert all(t for verses in chapters.values() for _, t in verses)
    result.append(dict(name=names[code], abbrev=code,
                       chapters=[[t for _, t in verses] for verses in chapters.values()],
                       labels=[[n for n, _ in verses] for verses in chapters.values()]))
assert len(result) == 81

# Charles 1917 public-domain translation: scripture body only. The printed
# edition groups some verses and places the Apocalypse of Weeks out of chapter
# order. Preserve labels and restore numeric chapter order without rewriting it.
soup = BeautifulSoup(args.enoch.read_bytes(), 'html.parser')
for tag in soup.select('.pageno,.pagenum,.footnote,a[role="doc-noteref"],a[href^="#f"],sup'):
    tag.decompose()
rows = collections.defaultdict(lambda: collections.OrderedDict())
started, current, chapter = False, None, None
roman_values = dict(I=1, V=5, X=10, L=50, C=100, D=500, M=1000)

def roman(text):
    values = [roman_values[c] for c in text]
    return sum(-v if i + 1 < len(values) and v < values[i + 1] else v for i, v in enumerate(values))

def append(text):
    if current and text.strip():
        rows[current[0]][current[1]].append(text.strip())

marker = re.compile(r'(?<![\w,])(?:([IVXLCDM]+)\.\s*)?(\d+(?:\s*[a-z])?(?:\s*[-–,]\s*\d+(?:\s*[a-z])?)*)\.\s+')
for node in soup.find_all(['h2', 'h3', 'h4', 'p', 'div']):
    if node.name == 'div' and 'line' not in node.get('class', []):
        continue
    if node.find_parent(class_='footnote') or node.find_parent(id='pg-footer'):
        continue
    text = node.get_text(' ', strip=True)
    if started and text.startswith('Printed in Great Britain'):
        break
    if not started:
        if 'The words of the blessing of Enoch' not in text:
            continue
        started = True
    if node.name.startswith('h'):
        if 'PROJECT GUTENBERG' in text.upper():
            break
        match = re.match(r'^([IVXLCDM]+)\.(?![-–,])', text)
        if match:
            chapter = roman(match[1])
        continue
    single = re.match(r'^([IVXLCDM]+)\.\s+(?![0-9])', text)
    if single:
        chapter = roman(single[1])
        current = (chapter, '1')
        rows[chapter].setdefault('1', [])
        text = text[single.end():]
    subverse = re.match(r'^([a-d])\.\s+', text)
    if subverse and current and current[0] == chapter:
        label = re.match(r'\d+', current[1])[0] + subverse[1]
        current = (chapter, label)
        rows[chapter].setdefault(label, [])
        text = text[subverse.end():]
    position = 0
    for match in marker.finditer(text):
        # 74:17 ends with the literal number "80." before the 75:1 marker.
        if chapter == 74 and match[2] == '80' and not match[1]:
            continue
        append(text[position:match.start()])
        if match[1]:
            chapter = roman(match[1])
        assert chapter, text[:100]
        label = re.sub(r'\s+', '', match[2]).replace('–', '-')
        current = (chapter, label)
        rows[chapter].setdefault(label, [])
        position = match.end()
    append(text[position:])
assert set(rows) == set(range(1, 109))
enoch = dict(name='1 Enoch (Charles 1917)', abbrev='ENO', chapters=[], labels=[])
for chapter in range(1, 109):
    labels = sorted(rows[chapter], key=lambda k: (int(re.match(r'\d+', k)[0]), k))
    verses = [' '.join(' '.join(rows[chapter][v]).split()) for v in labels]
    assert all(verses), chapter
    # Grouped verses must cover every source reference, with no invented gaps.
    covered = set()
    for label in labels:
        numbers = list(map(int, re.findall(r'\d+', label)))
        covered.update(range(numbers[0], numbers[-1] + 1))
    assert covered == set(range(1, max(covered) + 1)), (chapter, covered)
    enoch['chapters'].append(verses)
    enoch['labels'].append(labels)
assert enoch['labels'][59][-1] == '24' and enoch['labels'][89][-1] == '42', (enoch['labels'][59], enoch['labels'][89])
assert enoch['labels'][107][-1] == '15'
result.append(enoch)

version = 'The Book of Jubilees, trans. R. H. Charles. London [1917]'
jubilees = dict(name='Jubilees (Charles 1917)', abbrev='JUB', chapters=[])
jubilees_hashes = {}
for n in range(1, 51):
    raw = (args.jubilees_cache / f'{n}.json').read_bytes()
    source = json.loads(raw)
    assert source['versionTitle'] == version and source['text'], n
    verses = []
    for verse in source['text']:
        parsed = BeautifulSoup(verse, 'html.parser')
        assert not parsed.select('sup,.footnote'), (n, verse)
        verses.append(parsed.get_text().strip())
    assert all(verses), n
    jubilees['chapters'].append(verses)
    jubilees_hashes[str(n)] = hashlib.sha256(raw).hexdigest()
assert sum(map(len, jubilees['chapters'])) == 1758
result.append(jubilees)
assert len(result) == 83
root = Path(__file__).resolve().parents[1]
data = (json.dumps(result, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
(root / 'core/data/extended.json').write_bytes(data)
metadata = dict(name='Extended collection: WEB Classic + 1 Enoch + Jubilees', language='en',
                rights='Public domain source texts; see THIRD_PARTY_NOTICES.md and LICENSE.gutenberg.txt.',
                web=dict(source='https://ebible.org/Scriptures/eng-web_vpl.zip',
                         headers='https://ebible.org/Scriptures/eng-web_usfm.zip',
                         license_evidence='https://ebible.org/eng-web/copyright.htm',
                         source_sha256=hashlib.sha256(vpl.read_bytes()).hexdigest(),
                         headers_sha256=hashlib.sha256(usfm.read_bytes()).hexdigest(), books=81),
                enoch=dict(source='https://www.gutenberg.org/cache/epub/77935/pg77935-images.html',
                           translation='R. H. Charles, 1917', chapters=108,
                           source_sha256=hashlib.sha256(args.enoch.read_bytes()).hexdigest()),
                jubilees=dict(source='https://www.sefaria.org/api/texts/Book_of_Jubilees.{chapter}?context=0&ven=The%20Book%20of%20Jubilees%2C%20trans.%20R.%20H.%20Charles.%20London%20%5B1917%5D',
                              translation=version, license='Public Domain (Sefaria availableVersions metadata)',
                              chapters=50, chapter_sha256=jubilees_hashes),
                sha256=hashlib.sha256(data).hexdigest(), sections=83, retrieved='2026-10-24',
                conversion='WEB official XML text and source labels, empty records omitted, combined verse labels and gaps preserved. Charles texts: scripture body only, HTML/page/footnote markup and printer colophon removed, layout whitespace normalized, original chapter/verse order and combined labels retained; Jubilees uses source segment numbering. This is a collection, NOT a universal canon or the complete Ethiopian canon.')
metadata_path = root / 'core/data/extended-source.json'
if metadata_path.exists():
    expected = json.loads(metadata_path.read_text(encoding='utf-8'))
    assert metadata == expected, 'Source/output checksum mismatch; inspect before refreshing metadata.'
metadata_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'Extended collection: {len(result)} sections, {sum(len(b["chapters"]) for b in result)} chapters; SHA256 {metadata["sha256"]}')
