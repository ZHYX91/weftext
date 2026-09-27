import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("design_inputs", Path(__file__).resolve().parents[1] / "check_design_inputs.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class DesignInputTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / "docs/design"
        self.snap = self.base / "snapshots"
        self.snap.mkdir(parents=True)
        self.rows = []
        for i in range(1, 10):
            name = f"docs/design/snapshots/d{i}.md"
            (self.root / name).write_text("# Input\n", encoding="utf-8", newline="\n")
            self.rows.append({"path": name, "topic": f"D{i}", "main": True, "bytes": 8, "lines": 1})
        self.save()

    def save(self):
        (self.base / "inputs.json").write_text(json.dumps({"schema_version": 1, "status": "accepted-design-not-implemented", "files": self.rows}), encoding="utf-8")

    def test_complete_inventory(self):
        self.assertEqual(MODULE.validate(self.root), [])

    def test_unlisted_file_rejected(self):
        (self.snap / "extra.md").write_text("private", encoding="utf-8")
        self.assertTrue(any("explicit inventory" in x for x in MODULE.validate(self.root)))

    def test_truncated_input_rejected(self):
        (self.snap / "d1.md").write_text("", encoding="utf-8")
        self.assertTrue(any("length mismatch" in x for x in MODULE.validate(self.root)))

    def test_escaping_link_rejected(self):
        text = "[private](../../../../private.md)\n"
        (self.snap / "d1.md").write_text(text, encoding="utf-8", newline="\n")
        self.rows[0].update(bytes=len(text.encode()), lines=1)
        self.save()
        self.assertTrue(any("broken snapshot link" in x for x in MODULE.validate(self.root)))

    def test_missing_topic_rejected(self):
        self.rows[0]["main"] = False
        self.save()
        self.assertTrue(any("topic main" in x for x in MODULE.validate(self.root)))

    def test_private_chat_link_rejected(self):
        text = "https://chatgpt.com/c/private\n"
        (self.snap / "d1.md").write_text(text, encoding="utf-8", newline="\n")
        self.rows[0].update(bytes=len(text.encode()), lines=1)
        self.save()
        self.assertTrue(any("private transport" in x for x in MODULE.validate(self.root)))


if __name__ == "__main__":
    unittest.main()
