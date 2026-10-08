import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("check_docs", ROOT / "scripts/check_docs.py")
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)

INHERITED_FIELDS = ("id", "group", "positive", "negative", "contract", "evidence")
OVERLAY_FIELDS = ("id", "scenario", "positive", "negative", "contract", "status")
EN_INHERITED_HEADER = ("ID", "Group", "Positive", "Negative", "Contract", "Evidence")
EN_OVERLAY_HEADER = ("ID", "Scenario", "Positive", "Negative", "Current contract", "Evidence status")
ZH_INHERITED_HEADER = ("ID", "分组", "正向", "反向", "规范", "证据")
ZH_OVERLAY_HEADER = ("ID", "场景", "正向", "反向", "当前规范", "证据状态")


def inherited_row(row_id: str) -> dict[str, str]:
    return {
        "id": row_id,
        "group": f"group {row_id}",
        "positive": f"positive {row_id}",
        "negative": f"negative {row_id}",
        "contract": "D8§10",
        "evidence": "semantic-review",
    }


def overlay_row(row_id: str) -> dict[str, str]:
    return {
        "id": row_id,
        "scenario": f"scenario {row_id}",
        "positive": f"positive {row_id}",
        "negative": f"negative {row_id}",
        "contract": "D8§11",
        "status": "author-resolved-pending-independent-review",
    }


def escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace("|", "\\|")


def render_cells(cells: tuple[str, ...] | list[str]) -> str:
    return "| " + " | ".join(escape(value) for value in cells) + " |"


def render_row(value: dict[str, str], fields: tuple[str, ...]) -> str:
    return render_cells([value[field] for field in fields])


def table(header: tuple[str, ...], rows: list[dict[str, str]], fields: tuple[str, ...]) -> str:
    return "\n".join(
        [
            render_cells(header),
            "|---|---|---|---|---|---|",
            *(render_row(value, fields) for value in rows),
        ]
    )


class D8AcceptanceGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / "docs/design/a2-final-design"
        self.base.mkdir(parents=True)

        self.inherited = [inherited_row("VIEW-01"), inherited_row("VIEW-02"), inherited_row("VIEW-03")]
        self.inherited.extend(inherited_row(f"FIXED-{number:03d}") for number in range(1, 175))
        self.overlay = [overlay_row(f"VIEW-BLD-{number:02d}") for number in range(1, 11)]
        self.overlay.append(overlay_row("OVERLAY-01"))
        self.data = {
            "format": "weftext.a2-d8-acceptance",
            "version": 1,
            "status": "author-resolved-pending-independent-review",
            "fixedCaseCount": 160,
            "d6FaCaseCount": len(self.inherited),
            "inheritedCases": self.inherited,
            "currentOverlay": self.overlay,
            "counts": {
                "fixed": 160,
                "inherited": len(self.inherited),
                "currentOverlay": len(self.overlay),
                "totalCurrentObligations": len(self.inherited) + len(self.overlay),
                "uniqueIds": len(self.inherited) + len(self.overlay),
            },
            "structureAuthority": {
                "canonical": "D8-ACCEPTANCE.json",
                "projections": ["D8-ACCEPTANCE.md", "D8-ACCEPTANCE.zh-CN.md"],
                "rule": "test fixture",
            },
        }
        self.save_all()

    def write_json(self):
        (self.base / "D8-ACCEPTANCE.json").write_text(
            json.dumps(self.data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )

    def write_projections(self):
        english = (
            table(EN_INHERITED_HEADER, self.inherited, INHERITED_FIELDS)
            + "\n\n"
            + table(EN_OVERLAY_HEADER, self.overlay, OVERLAY_FIELDS)
            + "\n"
        )
        chinese = (
            table(ZH_INHERITED_HEADER, self.inherited, INHERITED_FIELDS)
            + "\n\n"
            + table(ZH_OVERLAY_HEADER, self.overlay, OVERLAY_FIELDS)
            + "\n"
        )
        (self.base / "D8-ACCEPTANCE.md").write_text(english, encoding="utf-8")
        (self.base / "D8-ACCEPTANCE.zh-CN.md").write_text(chinese, encoding="utf-8")

    def save_all(self):
        self.write_json()
        self.write_projections()

    def test_committed_canonical_acceptance(self):
        self.assertEqual(MODULE.validate_d8_acceptance(ROOT), [])

    def test_valid_dynamic_overlay_count(self):
        self.assertNotEqual(len(self.overlay), 36)
        self.assertEqual(MODULE.validate_d8_acceptance(self.root), [])

    def test_escaped_pipe_is_accepted(self):
        self.overlay[0]["positive"] = "positive with a|b"
        self.data["currentOverlay"] = self.overlay
        self.save_all()
        self.assertEqual(MODULE.validate_d8_acceptance(self.root), [])

    def test_malformed_json_fields_rejected(self):
        del self.data["currentOverlay"][0]["status"]
        self.write_json()
        self.assertTrue(
            any("must contain exactly" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_bare_pipe_rejected(self):
        path = self.base / "D8-ACCEPTANCE.zh-CN.md"
        text = path.read_text(encoding="utf-8")
        text = text.replace("positive VIEW-BLD-01", "positive VIEW-BLD-01 | broken", 1)
        path.write_text(text, encoding="utf-8")
        self.assertTrue(
            any("exactly six escaped cells" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_duplicate_json_id_rejected(self):
        self.data["currentOverlay"][1]["id"] = "VIEW-BLD-01"
        self.write_json()
        self.assertTrue(
            any("IDs must be unique" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_duplicate_projection_id_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        line = next(
            value for value in path.read_text(encoding="utf-8").splitlines()
            if value.startswith("| VIEW-BLD-01 |")
        )
        path.write_text(path.read_text(encoding="utf-8") + line + "\n", encoding="utf-8")
        self.assertTrue(
            any("duplicate D8 acceptance projection ID" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_unknown_projection_id_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        extra = render_cells(
            ["UNKNOWN-EXTRA", "extra", "positive", "negative", "D8§11", "pending"]
        )
        path.write_text(path.read_text(encoding="utf-8") + extra + "\n", encoding="utf-8")
        self.assertTrue(
            any("unknown D8 acceptance projection IDs" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_missing_projection_id_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        lines = [
            line for line in path.read_text(encoding="utf-8").splitlines()
            if not line.startswith("| VIEW-BLD-10 |")
        ]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assertTrue(
            any("missing D8 acceptance projection IDs" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_projection_order_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        lines = path.read_text(encoding="utf-8").splitlines()
        first = next(i for i, line in enumerate(lines) if line.startswith("| VIEW-01 |"))
        second = next(i for i, line in enumerate(lines) if line.startswith("| VIEW-02 |"))
        lines[first], lines[second] = lines[second], lines[first]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assertTrue(
            any("ID union/order" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_positive_negative_swap_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        lines = path.read_text(encoding="utf-8").splitlines()
        index = next(i for i, line in enumerate(lines) if line.startswith("| VIEW-BLD-01 |"))
        cells = MODULE._split_markdown_table_row(lines[index])
        self.assertIsNotNone(cells)
        cells[2], cells[3] = cells[3], cells[2]
        lines[index] = render_cells(cells)
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assertTrue(
            any("exact normalized six-field JSON projection" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_present_module_missing_canonical_json_rejected(self):
        (self.base / "D8-ACCEPTANCE.json").unlink()
        self.assertTrue(
            any("required D8 acceptance authority/projection file is missing" in error
                for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_present_module_missing_projection_rejected(self):
        (self.base / "D8-ACCEPTANCE.zh-CN.md").unlink()
        self.assertTrue(
            any("required D8 acceptance authority/projection file is missing" in error
                for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_absent_older_module_is_ignored(self):
        with tempfile.TemporaryDirectory() as temp:
            old_root = Path(temp)
            self.assertEqual(MODULE.validate_d8_acceptance(old_root), [])


if __name__ == "__main__":
    unittest.main()
