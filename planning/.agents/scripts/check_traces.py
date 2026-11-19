#!/usr/bin/env python3
"""工程痕跡檢查：讀者文件與程式碼裡不得出現流程痕跡。

在 repo 根目錄執行：python3 planning/.agents/scripts/check_traces.py
命中時逐行修掉，不加豁免。術語表的狀態欄按格式要求帶日期，所以只對術語表略過日期檢查。
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
PATTERNS = {
    "agent or tool name": re.compile(r"\b(Claude|Codex|ChatGPT|GPT-?\d[\w.]*|Copilot|Gemini|Cursor)\b"),
    "process term": re.compile(r"形狀卡|決策包|裁決|寫回|回執|統籌|子代理|審查線|審查包|待辦帳本|稿次"),
    "item number": re.compile(
        r"第\s*[0-9一二三四五六七八九十]+\s*(項|輪|批|稿|版)|(?<![\w.\-/#])\d{1,2}[a-e](?![\w.\-])"
    ),
    "words to the user": re.compile(r"請你|你可以|請使用者|請統籌|等你"),
}
DATE = re.compile(r"\b20\d{2}[-/年.]\d{1,2}")


def targets():
    docs = ROOT / "planning" / "docs"
    for path in sorted(docs.rglob("*.md")):
        if path.name != "AGENTS.md":
            yield path, "glossary" not in path.relative_to(docs).parts
    source = ROOT / "source"
    if source.exists():
        for pattern in ("**/*.rs", "**/*.md", "**/Cargo.toml"):
            for path in sorted(source.glob(pattern)):
                if "target" not in path.relative_to(source).parts:
                    yield path, True


def main() -> int:
    hits = []
    for path, check_dates in targets():
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            labels = [label for label, pattern in PATTERNS.items() if pattern.search(line)]
            if check_dates and DATE.search(line):
                labels.append("date")
            for label in labels:
                hits.append(f"{path.relative_to(ROOT)}:{number}: {label}: {line.strip()}")
    if hits:
        print("\n".join(hits))
        return 1
    print("engineering traces: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
