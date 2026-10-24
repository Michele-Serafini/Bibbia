"""Smoke test the compiled Rust ABI without a GUI or third-party Python packages."""
import ctypes
import json
import sys
import tempfile
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent.parent
    library = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "target/release/libbible_core.so"
    core = ctypes.CDLL(str(library.resolve()))
    core.bible_request.argtypes = [ctypes.c_char_p]
    core.bible_request.restype = ctypes.c_void_p
    core.bible_free.argtypes = [ctypes.c_void_p]
    core.bible_free.restype = None

    def call(**request):
        pointer = core.bible_request(json.dumps(request, ensure_ascii=False).encode())
        try:
            response = json.loads(ctypes.string_at(pointer).decode())
        finally:
            core.bible_free(pointer)
        if not response["ok"]:
            raise RuntimeError(response["error"])
        return response["data"]

    with tempfile.TemporaryDirectory(prefix="bibbia-smoke-") as directory:
        assert "KJV" in call(op="init", directory=directory)["editions"]
        assert len(call(op="books")) == 66
        assert len(call(op="chapter", book=0, chapter=0)) == 31
        hits = call(op="search", query="For God so loved the world")["hits"]
        assert any((v["book"], v["chapter"], v["verse"]) == (42, 2, 15) for v in hits)
        annotation = dict(highlight="green", underline=True, bookmark=True, note="Perché c’è speranza 🌱")
        call(op="annotate", book=42, chapter=2, verse=15, annotation=annotation)
        call(op="init", directory=directory)
        assert call(op="saved")[0]["annotation"] == {**annotation, "ranges": []}
        call(op="close")
        print("Release ABI: lettura, ricerca, UTF-8 e persistenza verificati.")


if __name__ == "__main__":
    main()
