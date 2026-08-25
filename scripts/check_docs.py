#!/usr/bin/env python3
"""Validate Weftext's paired public Markdown documentation."""

from __future__ import annotations

import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_PARTS = {"fixtures", "snapshots", "test-data"}
SPECIAL_GUIDES = {Path("LICENSE.zh-CN.md")}
TERMINOLOGY_GUIDES = {Path("docs/TERMINOLOGY.zh-CN.md")}

COMPARISON_NAMES = {
    "DocWen": re.compile(r"\bdocwen\b", re.IGNORECASE),
    "Obsidian": re.compile(r"\bobsidian\b", re.IGNORECASE),
    "Typora": re.compile(r"\btypora\b", re.IGNORECASE),
    "Notion": re.compile(r"\bnotion\b", re.IGNORECASE),
    "Feishu/Lark": re.compile(r"\bfeishu\b|\blark\b|飞书", re.IGNORECASE),
    "DingTalk": re.compile(r"\bdingtalk\b|钉钉", re.IGNORECASE),
}

HISTORICAL_LABELS = {
    "numbered Stage label": re.compile(r"\bStage\s+[0-9][A-Za-z0-9.-]*", re.IGNORECASE),
    "numbered Q label": re.compile(r"(?<![0-9-])\bQ[0-9](?:[-A-Za-z0-9.]*)\b"),
    "implementation checkpoint narrative": re.compile(
        r"implementation checkpoint|implemented checkpoint|dated checkpoint|"
        r"historical checkpoint|实现检查点|历史检查点|阶段性实现",
        re.IGNORECASE,
    ),
    "numbered package label": re.compile(r"\bpackage[- ]?[0-9]\b|包\s*[0-9]\b", re.IGNORECASE),
    "lettered release checkpoint": re.compile(r"\bR[0-9][A-Z]\b"),
    "dated baseline label": re.compile(
        r"\b20[0-9]{2}-[0-9]{2}-[0-9]{2}\s+baseline|"
        r"20[0-9]{2}-[0-9]{2}-[0-9]{2}\s*基线",
        re.IGNORECASE,
    ),
    "conversion-era narrative": re.compile(r"conversion-era|转换时代|转换时期", re.IGNORECASE),
}

FORBIDDEN_CHINESE_TERMS = {
    "workspace machine translation": re.compile(r"工作空间"),
    "product-surface machine translation": re.compile(
        r"产品表面|Query\s*表面|查询表面|嵌入式\s*Query\s*表面|"
        r"规范\s*Query\s*表面|Agent\s*表面|编辑表面|调用表面|输入表面|表面来源"
    ),
    "fail-closed machine translation": re.compile(r"失败关闭|封闭失败|无法关闭|关闭失败"),
    "worker machine translation": re.compile(r"工作器"),
    "draft machine translation": re.compile(r"草案"),
    "sidecar machine translation": re.compile(r"侧车|边车"),
    "native machine translation": re.compile(r"本机清单|本机表"),
    "promotion machine translation": re.compile(r"促销"),
    "Trash machine translation": re.compile(r"垃圾箱|垃圾载荷|垃圾项目|垃圾行|垃圾身份"),
    "caller machine translation": re.compile(r"检查员|呼叫者|调用者"),
    "authority machine translation": re.compile(r"物理权限|存储权限|名称权限|权限形状"),
    "release-edit machine translation": re.compile(
        r"源库存|广告为|后备行|奇偶性|释放转换|上演/移位|"
        r"模式关闭|关键权限|登录夹具|反弹配置|释放阻塞"
    ),
    "architecture-contract machine translation": re.compile(r"架构合同|当前合同|产品合同"),
}

LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+\S", re.MULTILINE)


def tracked_markdown() -> list[Path]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "*.md"],
        cwd=ROOT,
        text=True,
        encoding="utf-8",
    )
    paths = {Path(line.strip()) for line in output.splitlines() if line.strip()}
    return sorted(
        path
        for path in paths
        if (ROOT / path).is_file()
        and not any(part in FIXTURE_PARTS for part in path.parts)
    )


def parse_frontmatter(text: str) -> tuple[dict[str, str], str]:
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        return {}, text
    try:
        end = lines.index("---", 1)
    except ValueError:
        return {}, text
    values: dict[str, str] = {}
    for line in lines[1:end]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        values[key.strip()] = value.strip().strip("'\"")
    return values, "\n".join(lines[end + 1 :])


def without_fenced_code(text: str) -> str:
    kept: list[str] = []
    fence: str | None = None
    for line in text.splitlines():
        marker = re.match(r"^\s*(```+|~~~+)", line)
        if marker:
            token = marker.group(1)[0]
            if fence is None:
                fence = token
            elif fence == token:
                fence = None
            continue
        if fence is None:
            kept.append(line)
    return "\n".join(kept)


def headings(text: str) -> list[int]:
    visible = without_fenced_code(parse_frontmatter(text)[1])
    return [len(match.group(1)) for match in HEADING_RE.finditer(visible)]


def fenced_blocks(text: str) -> list[str]:
    body = parse_frontmatter(text)[1]
    blocks: list[str] = []
    current: list[str] | None = None
    fence: str | None = None
    for line in body.splitlines():
        marker = re.match(r"^\s*(```+|~~~+)", line)
        if marker:
            token = marker.group(1)[0]
            if fence is None:
                fence = token
                current = []
            elif fence == token:
                blocks.append("\n".join(current or []))
                fence = None
                current = None
            continue
        if current is not None:
            current.append(line)
    return blocks


def inline_code_spans(text: str) -> list[str]:
    visible = without_fenced_code(parse_frontmatter(text)[1])
    visible = re.sub(r"\[[^\]]*\]\([^)]+\)", "", visible)
    return re.findall(r"(?<!`)`([^`\r\n]+)`(?!`)", visible)


def check_chinese_prose(path: Path, text: str, errors: list[str]) -> None:
    if path.name == "THIRD_PARTY_NOTICES.zh-CN.md":
        return
    body = parse_frontmatter(text)[1]
    fence: str | None = None
    for line_number, line in enumerate(body.splitlines(), start=1):
        marker = re.match(r"^\s*(```+|~~~+)", line)
        if marker:
            token = marker.group(1)[0]
            if fence is None:
                fence = token
            elif fence == token:
                fence = None
            continue
        if fence is not None:
            continue
        prose = re.sub(r"`[^`]*`", "", line)
        prose = re.sub(r"\]\([^)]*\)", "]", prose)
        prose = re.sub(r"https?://\S+", "", prose)
        latin_words = re.findall(r"[A-Za-z]{3,}", prose)
        cjk_count = len(re.findall(r"[\u3400-\u9fff]", prose))
        if len(latin_words) >= 12 and cjk_count < len(latin_words):
            errors.append(
                f"{path}:{line_number}: Chinese source contains an untranslated or heavily mixed prose line"
            )


def check_chinese_terminology(path: Path, text: str, errors: list[str]) -> None:
    if path in TERMINOLOGY_GUIDES:
        return
    visible = without_fenced_code(parse_frontmatter(text)[1])
    visible = re.sub(r"`[^`]*`", "", visible)
    for label, pattern in FORBIDDEN_CHINESE_TERMS.items():
        match = pattern.search(visible)
        if match:
            line_number = visible.count("\n", 0, match.start()) + 1
            errors.append(
                f"{path}:{line_number}: Chinese source contains {label}: {match.group(0)}"
            )


def link_target(raw: str) -> str:
    target = raw.strip()
    if target.startswith("<") and ">" in target:
        return target[1 : target.index(">")]
    return target.split(maxsplit=1)[0]


def check_links(path: Path, text: str, errors: list[str]) -> None:
    visible = without_fenced_code(parse_frontmatter(text)[1])
    for match in LINK_RE.finditer(visible):
        raw_target = link_target(match.group(1))
        if not raw_target or raw_target.startswith(("#", "http://", "https://", "mailto:")):
            continue
        clean = unquote(raw_target.split("#", 1)[0].split("?", 1)[0])
        if not clean:
            continue
        resolved = (ROOT / path.parent / clean).resolve()
        try:
            resolved.relative_to(ROOT.resolve())
        except ValueError:
            errors.append(f"{path}: relative link escapes repository: {raw_target}")
            continue
        if not resolved.exists():
            errors.append(f"{path}: missing relative link target: {raw_target}")
            continue
        if path.name.endswith(".zh-CN.md") and resolved.suffix.lower() == ".md":
            english_pair = path.with_name(path.name.removesuffix(".zh-CN.md") + ".md")
            resolved_relative = resolved.relative_to(ROOT.resolve())
            if resolved_relative == english_pair:
                continue
            if not resolved.name.endswith(".zh-CN.md"):
                localized = resolved.with_name(resolved.stem + ".zh-CN.md")
                if localized.is_file():
                    errors.append(
                        f"{path}: link paired document through its Chinese source: {raw_target}"
                    )


def check_pair(english: Path, chinese: Path, errors: list[str]) -> None:
    english_text = (ROOT / english).read_text(encoding="utf-8")
    chinese_text = (ROOT / chinese).read_text(encoding="utf-8")
    english_meta, english_body = parse_frontmatter(english_text)
    chinese_meta, chinese_body = parse_frontmatter(chinese_text)

    if english_meta.get("source_language") != "zh-CN":
        errors.append(f"{english}: source_language must be zh-CN")
    if english_meta.get("translation_of") != chinese.name:
        errors.append(f"{english}: translation_of must be {chinese.name}")
    if english_meta.get("translation_status") != "synced":
        errors.append(f"{english}: translation_status must be synced")
    if chinese_meta != {"source_language": "zh-CN", "translation_status": "source"}:
        errors.append(f"{chinese}: source frontmatter must contain only source_language=zh-CN and translation_status=source")

    if f"[简体中文]({chinese.name})" not in english_body.splitlines()[:6]:
        errors.append(f"{english}: missing leading Simplified Chinese language link")
    if f"[English]({english.name})" not in chinese_body.splitlines()[:6]:
        errors.append(f"{chinese}: missing leading English language link")
    if headings(english_text) != headings(chinese_text):
        errors.append(f"{english} / {chinese}: heading levels differ")
    if len(fenced_blocks(english_text)) != len(fenced_blocks(chinese_text)):
        errors.append(f"{english} / {chinese}: fenced code block counts differ")
    if Counter(inline_code_spans(english_text)) != Counter(inline_code_spans(chinese_text)):
        errors.append(f"{english} / {chinese}: inline code values differ")
    english_links = LINK_RE.findall(without_fenced_code(english_body))
    chinese_links = LINK_RE.findall(without_fenced_code(chinese_body))
    if len(english_links) != len(chinese_links):
        errors.append(f"{english} / {chinese}: Markdown link counts differ")
    english_lines = max(1, len(english_body.splitlines()))
    chinese_lines = len(chinese_body.splitlines())
    if chinese_lines < english_lines * 0.5:
        errors.append(f"{chinese}: source is too short to be a complete translation")
    if len(chinese_body) > 200 and not re.search(r"[\u3400-\u9fff]", chinese_body):
        errors.append(f"{chinese}: source body has no Chinese text")

    check_chinese_prose(chinese, chinese_text, errors)

    check_links(english, english_text, errors)
    check_links(chinese, chinese_text, errors)


def main() -> int:
    errors: list[str] = []
    paths = tracked_markdown()
    path_set = set(paths)

    for path in paths:
        if path in SPECIAL_GUIDES:
            check_links(path, (ROOT / path).read_text(encoding="utf-8"), errors)
            continue
        if path.name.endswith(".zh-CN.md"):
            english = path.with_name(path.name.removesuffix(".zh-CN.md") + ".md")
            if english not in path_set:
                errors.append(f"{path}: missing English pair {english.name}")
            continue
        chinese = path.with_name(path.name.removesuffix(".md") + ".zh-CN.md")
        if chinese not in path_set:
            errors.append(f"{path}: missing Chinese source {chinese.name}")
            continue
        check_pair(path, chinese, errors)

    for path in paths:
        text = (ROOT / path).read_text(encoding="utf-8")
        if path.name.endswith(".zh-CN.md"):
            check_chinese_terminology(path, text, errors)
        for label, pattern in COMPARISON_NAMES.items():
            if pattern.search(text):
                errors.append(f"{path}: public documentation contains excluded comparison name {label}")
        for label, pattern in HISTORICAL_LABELS.items():
            if pattern.search(text):
                errors.append(f"{path}: public documentation contains {label}")

    if errors:
        print("Documentation check failed:", file=sys.stderr)
        for error in sorted(set(errors)):
            print(f"- {error}", file=sys.stderr)
        return 1
    print(f"Documentation check passed for {len(paths)} public Markdown files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
