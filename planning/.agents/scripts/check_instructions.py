#!/usr/bin/env python3
"""指令體系檢查：instructions 主題七項核對中可以機械化的部分。

在 repo 根目錄執行：python3 planning/.agents/scripts/check_instructions.py
腳本通過不代替人工核對，特別是「路由沒有形成第二份規範副本」。
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
PLANNING = ROOT / "planning"
TOPICS = PLANNING / ".agents" / "topics"
INDEX = PLANNING / ".agents" / "README.md"
ROUTE_LIMITS = {
    ROOT / "AGENTS.md": 8,
    PLANNING / "AGENTS.md": 40,
    PLANNING / "docs" / "AGENTS.md": 12,
}
TOPIC_REF = re.compile(r"topics/([\w-]+\.md)")
LINK = re.compile(r"\]\(([^)\s]+)")


def rel(path: Path) -> Path:
    return path.relative_to(ROOT)


def main() -> int:
    errors = []
    for route, limit in ROUTE_LIMITS.items():
        if not route.exists():
            errors.append(f"missing route {rel(route)}")
        elif len(route.read_text(encoding="utf-8").splitlines()) > limit:
            errors.append(f"{rel(route)}: longer than {limit} lines; routes hold pointers only")

    topics = {path.name for path in TOPICS.glob("*.md")}
    index = INDEX.read_text(encoding="utf-8") if INDEX.exists() else ""
    for name in sorted(topics):
        if f"topics/{name}" not in index:
            errors.append(f"topic {name} is not listed in {rel(INDEX)}")

    referenced = set()
    for route in ROUTE_LIMITS:
        if route.exists():
            for name in TOPIC_REF.findall(route.read_text(encoding="utf-8")):
                referenced.add(name)
                if name not in topics:
                    errors.append(f"{rel(route)} references missing topic {name}")
    for name in sorted(topics - referenced):
        errors.append(f"topic {name} is not referenced by any route")

    for readme in sorted((PLANNING / "docs").rglob("README.md")):
        for target in LINK.findall(readme.read_text(encoding="utf-8")):
            if "AGENTS.md" in target or ".agents" in target:
                errors.append(f"{rel(readme)} links instruction file {target}")

    root = ROOT / "AGENTS.md"
    if root.exists():
        body = [line for line in root.read_text(encoding="utf-8").splitlines() if line.strip() and not line.startswith("#")]
        if len(body) != 3 or not all(line.startswith("- ") for line in body):
            errors.append("AGENTS.md must hold exactly three pointer bullets and nothing else")

    if errors:
        print("\n".join(errors))
        return 1
    print(f"instruction system: PASS ({len(topics)} topics)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
