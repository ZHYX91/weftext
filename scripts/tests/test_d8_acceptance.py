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

FIELDS = ("id", "scenario", "positive", "negative", "contract", "status")


def row(row_id: str) -> dict[str, str]:
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


def render(value: dict[str, str]) -> str:
    return "| " + " | ".join(escape(value[field]) for field in FIELDS) + " |"


class D8AcceptanceGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / "docs/design/a2-final-design"
        self.base.mkdir(parents=True)

        inherited = [row("VIEW-01"), row("VIEW-02"), row("VIEW-03")]
        inherited.extend(row(f"FIXED-{number:03d}") for number in range(1, 175))
        overlay = [row(f"VIEW-BLD-{number:02d}") for number in range(1, 11)]
        overlay.extend(row(f"OVERLAY-{number:02d}") for number in range(1, 27))
        self.data = {
            "format": "weftext.a2-d8-acceptance",
            "version": 1,
            "status": "author-resolved-pending-independent-review",
            "inheritedCases": inherited,
            "currentOverlay": overlay,
            "counts": {
                "inherited": len(inherited),
                "currentOverlay": len(overlay),
                "totalCurrentObligations": len(inherited) + len(overlay),
                "uniqueIds": len(inherited) + len(overlay),
            },
            "structureAuthority": {
                "canonical": "D8-ACCEPTANCE.json",
                "projections": ["D8-ACCEPTANCE.md", "D8-ACCEPTANCE.zh-CN.md"],
                "rule": "test fixture",
            },
        }
        self.save()

    def save(self):
        (self.base / "D8-ACCEPTANCE.json").write_text(
            json.dumps(self.data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        rows = [*self.data["inheritedCases"], *self.data["currentOverlay"]]
        table = "\n".join(render(value) for value in rows) + "\n"
        (self.base / "D8-ACCEPTANCE.md").write_text(table, encoding="utf-8")
        (self.base / "D8-ACCEPTANCE.zh-CN.md").write_text(table, encoding="utf-8")

    def test_valid_current_tables(self):
        self.assertEqual(MODULE.validate_d8_acceptance(self.root), [])

    def test_malformed_json_fields_rejected(self):
        del self.data["currentOverlay"][0]["status"]
        self.save()
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

    def test_duplicate_id_rejected(self):
        self.data["currentOverlay"][1]["id"] = "VIEW-BLD-01"
        self.save()
        self.assertTrue(
            any("IDs must be unique" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_projection_order_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        lines = path.read_text(encoding="utf-8").splitlines()
        lines[0], lines[1] = lines[1], lines[0]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assertTrue(
            any("ID order" in error for error in MODULE.validate_d8_acceptance(self.root))
        )

    def test_positive_negative_swap_rejected(self):
        path = self.base / "D8-ACCEPTANCE.md"
        lines = path.read_text(encoding="utf-8").splitlines()
        cells = MODULE._split_markdown_table_row(lines[0])
        self.assertIsNotNone(cells)
        cells[2], cells[3] = cells[3], cells[2]
        lines[0] = "| " + " | ".join(escape(value) for value in cells) + " |"
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assertTrue(
            any("exact six-field JSON projection" in error for error in MODULE.validate_d8_acceptance(self.root))
        )


if __name__ == "__main__":
    unittest.main()
