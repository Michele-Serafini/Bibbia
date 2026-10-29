#!/usr/bin/env python3
"""Validate a native-rendered multipage fixture with real Unicode text extraction."""
from pathlib import Path
import sys
from pypdf import PdfReader
path=Path(sys.argv[1])
assert path.read_bytes().startswith(b'%PDF-'), path
reader=PdfReader(path)
assert len(reader.pages)>=3, len(reader.pages)
text='\n'.join(page.extract_text() or '' for page in reader.pages)
assert '000' in text and '359' in text, text[-1500:]
assert 'perché' in text and 'Ω' in text, text[:1500]
print(f'Native PDF: {len(reader.pages)} readable pages, first/last lines and Unicode verified.')
