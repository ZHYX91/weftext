#!/usr/bin/env python3
"""Check the explicit imported design inventory without treating it as shipped code."""
from __future__ import annotations

import json
import re
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]


def validate(root: Path) -> list[str]:
    errors = []
    base = root / "docs/design"
    inventory = json.loads((base / "inputs.json").read_text(encoding="utf-8"))
    if inventory.get("schema_version") != 1 or inventory.get("status") != "accepted-design-not-implemented":
        errors.append("unexpected inventory schema/status")
    entries = inventory["files"]
    paths = [row["path"] for row in entries]
    if len(paths) != len(set(paths)):
        errors.append("duplicate input path")
    actual = {p.relative_to(root).as_posix() for p in (base / "snapshots").rglob("*") if p.is_file()}
    if actual != set(paths):
        errors.append("snapshot files differ from the explicit inventory")
    mains = {row["topic"] for row in entries if row.get("main")}
    if mains != {f"D{i}" for i in range(1, 10)}:
        errors.append("missing or unexpected topic main documents")
    for row in entries:
        p = (root / row["path"]).resolve()
        if not p.is_relative_to((base / "snapshots").resolve()):
            errors.append(f"input outside snapshots: {row['path']}")
            continue
        if not p.is_file():
            errors.append(f"missing input: {row['path']}")
            continue
        raw = p.read_bytes()
        text = raw.decode("utf-8")
        if len(raw) != row["bytes"] or len(text.splitlines()) != row["lines"]:
            errors.append(f"inventory length mismatch: {row['path']}")
        if re.search(r"https://chatgpt\.com/c/|[A-Za-z]:[\\/]Users[\\/]|gh[pousr]_[A-Za-z0-9]{20,}", text):
            errors.append(f"private transport material: {row['path']}")
        if p.suffix == ".json":
            json.loads(text)
            continue
        for target in re.findall(r"(?<!!)\[[^\]]*\]\(([^)]+)\)", text):
            if target.startswith(("#", "https://", "http://", "mailto:")):
                continue
            file = unquote(target.split("#", 1)[0].strip("<>"))
            resolved = (p.parent / file).resolve()
            if not resolved.is_relative_to(root.resolve()) or not resolved.exists():
                errors.append(f"broken snapshot link: {row['path']} -> {target}")
    return errors


if __name__ == "__main__":
    problems = validate(ROOT)
    for problem in problems:
        print(problem)
    print(f"Design input check: {'FAIL' if problems else 'PASS'} (inventory and transport, not semantic acceptance)")
    raise SystemExit(bool(problems))
