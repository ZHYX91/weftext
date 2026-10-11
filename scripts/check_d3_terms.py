#!/usr/bin/env python3
"""Compare the complete current D3 terminology projection with its human source."""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIELDS = (
    ("定义", "currentDefinition"),
    ("owner/layer", "currentOwnerLayer"),
    ("firstFreeze", "firstFreeze"),
)


def fold(text: str) -> str:
    return " ".join(text.split())


def project(source: str) -> list[dict]:
    rows = []
    pattern = r"^### (weftext\.term\.[^\s]+)\s*\n(.*?)(?=^### |\Z)"
    for match in re.finditer(pattern, source, re.M | re.S):
        concept_id, block = match.groups()
        row = {"conceptId": concept_id}
        for label, key in FIELDS:
            field_pattern = r"^- " + re.escape(label) + r"：(.*?)(?=^\s*- [^\n：]+：|\Z)"
            field = re.search(field_pattern, block, re.M | re.S)
            if not field:
                raise ValueError(f"{concept_id}: missing {label}")
            value = fold(field.group(1))
            row[key] = value.removesuffix("。") if key == "firstFreeze" else value
        controlled = re.search(r"~~~json\s*\n(.*?)\n~~~", block, re.S)
        if not controlled:
            raise ValueError(f"{concept_id}: missing controlled sets")
        row.update(json.loads(controlled.group(1)))
        rows.append(row)
    if not rows or len({row["conceptId"] for row in rows}) != len(rows):
        raise ValueError("missing/duplicate concepts")
    return rows


def compare(machine: dict, source: str) -> list[str]:
    human = project(source)
    rows = machine["concepts"]
    errors = []
    if machine["concept_count"] != len(rows) or [row["conceptId"] for row in rows] != [row["conceptId"] for row in human]:
        errors.append("concept count/order mismatch")
    keys = ("conceptId", "currentDefinition", "currentOwnerLayer", "firstFreeze", "ownedNames", "semanticNonaliases")
    for machine_row, human_row in zip(rows, human):
        for key in keys:
            actual = machine_row.get(key)
            expected = human_row.get(key)
            if key in ("currentDefinition", "currentOwnerLayer"):
                actual = fold(actual or "")
            if key == "firstFreeze":
                # Legacy machine fields append migration notes after a semicolon;
                # the human source keeps those notes in its separate migration field.
                actual = (actual or "").replace("`", "").split(";", 1)[0]
            if actual != expected:
                errors.append(f"{machine_row['conceptId']}: {key} full projection mismatch")
    return errors


def validate(root: Path) -> list[str]:
    base = root / "docs/design/a2-final-design"
    machine = json.loads((base / "D3-TERMS.json").read_text(encoding="utf-8-sig"))
    source = (base / "D3-LEXICON.zh-CN.md").read_text(encoding="utf-8-sig")
    return compare(machine, source)


if __name__ == "__main__":
    problems = validate(ROOT)
    for problem in problems:
        print(problem)
    print(f"D3 complete terminology projection: {'FAIL' if problems else 'PASS'}")
    raise SystemExit(bool(problems))
