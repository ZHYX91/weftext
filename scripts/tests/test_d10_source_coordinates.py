"""Guard historical D10 coordinates and both current bilingual Markdown projections.

This tests the five source inputs and the current A2 scenario presentation without
creating another normative scenario inventory. Mutants use the real repository rows.
"""

import copy
import hashlib
import json
import unittest
from pathlib import Path, PureWindowsPath


ROOT = Path(__file__).resolve().parents[2]
HIST = ROOT / "docs/design/d10"
A2 = ROOT / "docs/design/a2-final-design"
BLOBS = {
    "en": ("SCENARIO-DISPOSITIONS.md", "f7bd02712e17f39eb5d09038a7b565d42ad70dae"),
    "zh": ("SCENARIO-DISPOSITIONS.zh-CN.md", "21d318c1ad7a34852163bb0967f820ae84b74a4d"),
}
PROJECTED = {"en": "D10-ACCEPTANCE.md", "zh": "D10-ACCEPTANCE.zh-CN.md"}
CURRENT = {"en": "D10-SCENARIOS.md", "zh": "D10-SCENARIOS.zh-CN.md"}
HEADERS = {
    "en": ("ID", "Source", "Disposition", "Mode", "Positive/expected", "Negative/risk", "Validation oracle"),
    "zh": ("ID", "来源", "处置", "执行", "正向规则", "反向风险", "验收证据义务"),
}


def git_blob_sha(data: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()


def cells(line: str) -> list[str]:
    """Split a Markdown table row on unescaped pipes outside inline code spans.

    Table escaping is presentation-only: decode \| outside code to the literal |
    but keep backticks and code-span content intact for byte-precise oracles.
    """
    if not line.startswith("|") or not line.endswith("|"):
        raise ValueError("table row must have boundary pipes")
    result, part, code, index = [], [], None, 0
    while index < len(line):
        char = line[index]
        if char == "\\" and code is None and index + 1 < len(line) and line[index + 1] in "|\\`":
            part.append(line[index + 1])
            index += 2
        elif char == "`":
            end = index
            while end < len(line) and line[end] == "`":
                end += 1
            run = line[index:end]
            if code is None:
                code = run
            elif run == code:
                code = None
            part.append(run)
            index = end
        elif char == "|" and code is None:
            result.append("".join(part).strip())
            part.clear()
            index += 1
        else:
            part.append(char)
            index += 1
    if code is not None or part or not result or result[0] != "":
        raise ValueError("malformed Markdown table row or unclosed inline code")
    return result[1:]


def scenario_rows(text: str) -> list[list[str]]:
    return [cells(line) for line in text.splitlines() if line.startswith("| D10-")]


def acceptance_rows(text: str, language: str) -> list[list[str]]:
    lines = text.splitlines()
    header = "| " + " | ".join(HEADERS[language]) + " |"
    if lines.count(header) != 1:
        raise AssertionError(f"{language}: one seven-column acceptance header required")
    start = lines.index(header)
    if start + 1 >= len(lines) or not lines[start + 1].startswith("| ---"):
        raise AssertionError(f"{language}: acceptance table separator missing")
    output = []
    for line in lines[start + 2 :]:
        if not line.startswith("|"):
            break
        output.append(cells(line))
    return output


def replace_cell(text: str, case_id: str, position: int, value: str) -> str:
    lines = text.splitlines(keepends=True)
    matches = [i for i, line in enumerate(lines) if line.startswith(f"| {case_id} |")]
    if len(matches) != 1:
        raise AssertionError(f"mutant fixture requires one {case_id}")
    index = matches[0]
    old = cells(lines[index].rstrip("\r\n"))
    old[position] = value
    lines[index] = "| " + " | ".join(old) + " |\n"
    return "".join(lines)


def change_row_count(text: str, case_id: str, kind: str) -> str:
    lines = text.splitlines(keepends=True)
    matches = [i for i, line in enumerate(lines) if line.startswith(f"| {case_id} |")]
    if len(matches) != 1:
        raise AssertionError("mutant fixture row missing")
    if kind == "duplicate":
        lines.insert(matches[0], lines[matches[0]])
    elif kind == "delete":
        lines.pop(matches[0])
    elif kind == "reorder":
        lines[matches[0]], lines[matches[0] + 1] = lines[matches[0] + 1], lines[matches[0]]
    else:
        raise ValueError("unknown fixture mutation")
    return "".join(lines)


class D10SourceCoordinates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = {
            "source": {lang: (HIST / name).read_bytes() for lang, (name, _) in BLOBS.items()},
            "acceptance": json.loads((A2 / "D10-ACCEPTANCE.json").read_text(encoding="utf-8")),
            "projected": {lang: (A2 / name).read_text(encoding="utf-8") for lang, name in PROJECTED.items()},
            "current": {lang: (A2 / name).read_text(encoding="utf-8") for lang, name in CURRENT.items()},
        }

    def validate(self, fixture):
        document = fixture["acceptance"]  # the one structural acceptance authority
        rows = document["rows"]
        ids = [item["id"] for item in rows]
        self.assertEqual(125, len(rows), "125 stable original scenario IDs")
        self.assertEqual(125, len(set(ids)), "duplicate JSON scenario ID")
        for language, (source_file, sha) in BLOBS.items():
            source_path = HIST / source_file
            source = fixture["source"][language]
            self.assertEqual(sha, git_blob_sha(source), f"{language}: immutable historical blob")
            numbered = [
                (index, line, cells(line))
                for index, line in enumerate(source.decode("utf-8").splitlines(), 1)
                if line.startswith("| D10-")
            ]
            historical = [fields for _, _, fields in numbered]
            projected = acceptance_rows(fixture["projected"][language], language)
            current = scenario_rows(fixture["current"][language])
            for name, table in (("historical", historical), ("acceptance", projected), ("current", current)):
                self.assertEqual(125, len(table), f"{language} {name}: missing or extra row")
                self.assertTrue(all(len(fields) == 7 for fields in table), f"{language} {name}: column count")
                self.assertEqual(125, len({fields[0] for fields in table}), f"{language} {name}: duplicate ID")
                self.assertEqual(ids, [fields[0] for fields in table], f"{language} {name}: original ID ordering")
            for index, row in enumerate(rows):
                key = row["id"]
                line_number, raw, original = numbered[index]
                record = row["source"][language]
                # Path.as_posix works on both PosixPath and WindowsPath.
                self.assertEqual(source_path.relative_to(ROOT).as_posix(), record["path"])
                self.assertEqual(sha, record["blob"])
                self.assertEqual(line_number, record["line"])
                self.assertEqual(raw, record["raw"])
                original_expected = [
                    key, record["location"], row["negative"][language],
                    row["disposition"][language], row["executionMode"][language],
                    row["positive"][language], row["oracle"][language],
                ]
                self.assertEqual(original_expected, original, "original seven source cells")
                projected_expected = [
                    key, record["location"], row["disposition"][language],
                    row["executionMode"][language], row["positive"][language],
                    row["negative"][language], row["oracle"][language],
                ]
                for column, (actual, expected) in enumerate(zip(projected[index], projected_expected)):
                    self.assertEqual(expected, actual, f"{language} {key}: acceptance column {column}")
                current_expected_unchanged = [
                    key, None, row["negative"][language], row["disposition"][language],
                    row["executionMode"][language], None, row["oracle"][language],
                ]
                for column in (0, 2, 3, 4, 6):
                    self.assertEqual(
                        current_expected_unchanged[column], current[index][column],
                        f"{language} {key}: current scenario column {column}",
                    )
            # Overlay is derived from the *actual current display* against the
            # historical cells. No copy of scenario meanings or second map.
            for index, row in enumerate(rows):
                original, displayed = historical[index], current[index]
                shown_delta = {}
                if displayed[1] != original[1]:
                    shown_delta["location"] = displayed[1]
                if displayed[5] != original[5]:
                    shown_delta["positive"] = displayed[5]
                overlay = row.get("currentOverlay") or {}
                retained = overlay.get("sourceEraToCurrent") or {}
                self.assertEqual(
                    shown_delta, retained.get(language, {}),
                    f"{language} {row['id']}: actual current overlay display drift",
                )
        for row in rows:
            overlay = row.get("currentOverlay") or {}
            actual = overlay.get("sourceEraToCurrent") or {}
            if actual:
                self.assertEqual({"en", "zh"}, set(actual), f"{row['id']}: bilingual current overlay")
                self.assertIs(overlay.get("historicalSourceRetained"), True)

    def test_real_original_and_current_bilingual_tables(self):
        self.validate(self.fixture)

    def test_windows_path_flavor_is_posix_in_source_metadata(self):
        self.assertEqual(
            "docs/design/d10/SCENARIO-DISPOSITIONS.md",
            PureWindowsPath(r"docs\design\d10\SCENARIO-DISPOSITIONS.md").as_posix(),
        )

    def test_escaped_pipes_and_inline_code_pipes(self):
        row = r"| D10-X99 | alpha \| beta and `code|pipe` | reject | automatic | `a|b` | hidden \| input | oracle |"
        self.assertEqual(
            ["D10-X99", "alpha | beta and `code|pipe`", "reject", "automatic", "`a|b`", "hidden | input", "oracle"],
            cells(row),
        )
        with self.assertRaises(ValueError):
            cells("| D10-X99 | `unclosed code | reject |")

    def test_real_acceptance_column_mutants_in_both_languages(self):
        for lang in BLOBS:
            for column in range(7):
                with self.subTest(language=lang, column=column):
                    mutant = copy.deepcopy(self.fixture)
                    mutant["projected"][lang] = replace_cell(
                        mutant["projected"][lang], "D10-U01", column, f"WRONG_{lang}_{column}",
                    )
                    with self.assertRaises(AssertionError):
                        self.validate(mutant)

    def test_real_duplicate_missing_and_reordered_projection_rows(self):
        for lang in BLOBS:
            for kind in ("duplicate", "delete", "reorder"):
                with self.subTest(language=lang, kind=kind):
                    mutant = copy.deepcopy(self.fixture)
                    mutant["projected"][lang] = change_row_count(
                        mutant["projected"][lang], "D10-U01", kind,
                    )
                    with self.assertRaises(AssertionError):
                        self.validate(mutant)

    def test_real_duplicate_missing_and_reordered_current_rows(self):
        for lang in BLOBS:
            for kind in ("duplicate", "delete", "reorder"):
                with self.subTest(language=lang, kind=kind):
                    mutant = copy.deepcopy(self.fixture)
                    mutant["current"][lang] = change_row_count(mutant["current"][lang], "D10-U01", kind)
                    with self.assertRaises(AssertionError):
                        self.validate(mutant)

    def test_real_current_overlay_and_scenario_display_mutants(self):
        for lang in BLOBS:
            for case_id, position in (("D10-F01", 1), ("D10-U27", 5)):
                with self.subTest(language=lang, case=case_id):
                    mutant = copy.deepcopy(self.fixture)
                    mutant["current"][lang] = replace_cell(
                        mutant["current"][lang], case_id, position, "WRONG_DISPLAY",
                    )
                    with self.assertRaises(AssertionError):
                        self.validate(mutant)
                    mutant = copy.deepcopy(self.fixture)
                    field = "location" if position == 1 else "positive"
                    match = next(row for row in mutant["acceptance"]["rows"] if row["id"] == case_id)
                    match["currentOverlay"]["sourceEraToCurrent"][lang][field] = "WRONG_OVERLAY"
                    with self.assertRaises(AssertionError):
                        self.validate(mutant)

    def test_real_historical_source_path_mutant(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["acceptance"]["rows"][0]["source"]["en"]["path"] = "docs\\design\\d10\\SCENARIO-DISPOSITIONS.md"
        with self.assertRaises(AssertionError):
            self.validate(mutant)


if __name__ == "__main__":
    unittest.main()
