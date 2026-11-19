#!/usr/bin/env python3
"""讀者文件檢查：一級標題、歸屬中繼資料、本地連結與錨點、索引覆蓋、不連結指令檔。

在 repo 根目錄執行：python3 planning/.agents/scripts/check_reader_docs.py
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
DOCS = ROOT / "planning" / "docs"
LINK = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
EXTERNAL = ("http://", "https://", "mailto:")
METADATA = ("> 文件歸屬：", "> 涉及主體：")


def prose(text: str) -> str:
    """Drop fenced code blocks so examples are not checked as content."""
    return re.sub(r"^```.*?^```", "", text, flags=re.S | re.M)


def anchors(text: str) -> set[str]:
    found = set()
    for line in prose(text).splitlines():
        match = re.match(r"^#{1,6}\s+(.+?)\s*$", line)
        if match:
            found.add(re.sub(r"[^\w\- ]", "", match.group(1).lower()).replace(" ", "-"))
    return found


def check_document(doc: Path, errors: list[str]) -> None:
    rel = doc.relative_to(ROOT)
    lines = prose(doc.read_text(encoding="utf-8")).splitlines()
    h1 = [i for i, line in enumerate(lines) if line.startswith("# ")]
    if len(h1) != 1:
        errors.append(f"{rel}: expected one H1, found {len(h1)}")
    else:
        following = [line for line in lines[h1[0] + 1 :] if line.strip()][:2]
        if [line[: len(key)] for line, key in zip(following, METADATA)] != list(METADATA):
            errors.append(f"{rel}: the two metadata lines must directly follow the H1")
    for key in METADATA:
        count = sum(1 for line in lines if line.startswith(key))
        if count != 1:
            errors.append(f"{rel}: expected one '{key}' line, found {count}")
    for target in LINK.findall("\n".join(lines)):
        if target.startswith(EXTERNAL):
            continue
        path, _, anchor = target.partition("#")
        if "AGENTS.md" in path or ".agents" in path:
            errors.append(f"{rel}: reader document links an instruction file -> {target}")
        dest = (doc.parent / path).resolve() if path else doc.resolve()
        if not dest.exists():
            errors.append(f"{rel}: broken link -> {target}")
        elif anchor and dest.suffix == ".md" and anchor not in anchors(dest.read_text(encoding="utf-8")):
            errors.append(f"{rel}: broken anchor -> {target}")


def check_index(doc: Path, errors: list[str]) -> None:
    if doc == DOCS / "README.md":
        return
    index = doc.parent.parent / "README.md" if doc.name == "README.md" else doc.parent / "README.md"
    if not index.exists():
        errors.append(f"{doc.relative_to(ROOT)}: missing index {index.relative_to(ROOT)}")
        return
    linked = {
        (index.parent / target.partition("#")[0]).resolve()
        for target in LINK.findall(prose(index.read_text(encoding="utf-8")))
        if not target.startswith(EXTERNAL)
    }
    if doc.resolve() not in linked:
        errors.append(f"{doc.relative_to(ROOT)}: not linked from {index.relative_to(ROOT)}")


def main() -> int:
    errors: list[str] = []
    docs = sorted(path for path in DOCS.rglob("*.md") if path.name != "AGENTS.md")
    for doc in docs:
        check_document(doc, errors)
        check_index(doc, errors)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"reader documents: PASS ({len(docs)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
