#!/usr/bin/env python3
"""術語表檢查：格式、狀態取值、名字唯一、總入口涵蓋分表；另列出 source/ 裡不在表中的公開名字。

在 repo 根目錄執行：python3 planning/.agents/scripts/check_glossary.py
只有術語表本身的格式問題會讓檢查失敗；不在表中的名字只列清單，由人判斷是否承載概念。
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
GLOSSARY = ROOT / "planning" / "docs" / "glossary"
HEADER = ["中文術語", "英文名／識別符", "概念", "來源", "狀態", "規範位置", "別名與舊叫法"]
SECTIONS = ["## 本專案自造", "## 外部既有"]
NOT_TABLES = {"README.md", "conflicts.md"}
PUBLIC_ITEM = re.compile(r"^\s*pub\s+(?:struct|enum|trait|type|fn|const)\s+([A-Za-z_][A-Za-z0-9_]*)", re.M)
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def cells(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def check_table(path: Path, names: dict[str, str], known: set[str], errors: list[str]) -> None:
    rel = path.relative_to(ROOT)
    lines = path.read_text(encoding="utf-8").splitlines()
    starts = [lines.index(section) if section in lines else -1 for section in SECTIONS]
    if -1 in starts or starts != sorted(starts):
        errors.append(f"{rel}: needs the sections {SECTIONS} in this order")
        return
    for start, section in zip(starts, SECTIONS):
        end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
        rows = [line for line in lines[start + 1 : end] if line.startswith("|")]
        if not rows or cells(rows[0]) != HEADER:
            errors.append(f"{rel} {section}: header must be {' | '.join(HEADER)}")
            continue
        for row in rows[2:]:
            values = cells(row)
            if len(values) != 7:
                errors.append(f"{rel}: row needs 7 cells: {row}")
                continue
            name, source, status, aliases = values[1].strip("`"), values[3], values[4], values[6]
            if not name:
                errors.append(f"{rel}: the English name is the primary key and cannot be empty: {row}")
            if not status.startswith(("已確認", "建議待確認")):
                errors.append(f"{rel}: status of {name} must start with 已確認 or 建議待確認")
            if section == SECTIONS[0] and source != "自造":
                errors.append(f"{rel}: self-made entry {name} needs source 自造")
            if section == SECTIONS[1] and not source.startswith("外部"):
                errors.append(f"{rel}: external entry {name} needs a source starting with 外部")
            if name in names:
                errors.append(f"{rel}: {name} already has an entry in {names[name]}")
            names[name] = path.name
            known.add(name)
            known.update(IDENTIFIER.findall(aliases))


def main() -> int:
    errors: list[str] = []
    names: dict[str, str] = {}
    known: set[str] = set()
    entry = (GLOSSARY / "README.md").read_text(encoding="utf-8")
    for path in sorted(GLOSSARY.glob("*.md")):
        if path.name != "README.md" and f"]({path.name})" not in entry:
            errors.append(f"glossary entry point does not link {path.name}")
        if path.name not in NOT_TABLES:
            check_table(path, names, known, errors)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"glossary: PASS ({len(names)} entries)")
    source = ROOT / "source" / "src"
    if source.exists():
        public = set()
        for path in source.rglob("*.rs"):
            public.update(PUBLIC_ITEM.findall(path.read_text(encoding="utf-8")))
        missing = sorted(public - known)
        if missing:
            print("public names not in the glossary (decide whether each carries a concept):")
            print("\n".join(f"  {name}" for name in missing))
    return 0


if __name__ == "__main__":
    sys.exit(main())
