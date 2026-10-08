"""Focused D10 historical source coordinates, stable IDs, and current-overlay separation."""
import hashlib
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HIST = ROOT / "docs/design/d10"
A2 = ROOT / "docs/design/a2-final-design"
BLOBS = {
    "en": ("SCENARIO-DISPOSITIONS.md", "f7bd02712e17f39eb5d09038a7b565d42ad70dae"),
    "zh": ("SCENARIO-DISPOSITIONS.zh-CN.md", "21d318c1ad7a34852163bb0967f820ae84b74a4d"),
}
PROJECTED = {"en": "D10-ACCEPTANCE.md", "zh": "D10-ACCEPTANCE.zh-CN.md"}


def git_blob_sha(data: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()


class D10SourceCoordinates(unittest.TestCase):
    def test_original_two_blobs_all_125_rows_and_projection(self):
        rows = json.loads((A2 / "D10-ACCEPTANCE.json").read_text(encoding="utf-8"))["rows"]
        ids = [row["id"] for row in rows]
        self.assertEqual(125, len(ids))
        self.assertEqual(125, len(set(ids)))
        for language, (source_file, sha) in BLOBS.items():
            source_path = HIST / source_file
            source_bytes = source_path.read_bytes()
            self.assertEqual(sha, git_blob_sha(source_bytes))
            lines = source_bytes.decode("utf-8").splitlines()
            found = {}
            for line_number, raw in enumerate(lines, 1):
                if raw.startswith("| D10-"):
                    row_id = raw.split("|", 2)[1].strip()
                    self.assertNotIn(row_id, found)
                    found[row_id] = (line_number, raw)
            self.assertEqual(set(ids), set(found))
            current_table = (A2 / PROJECTED[language]).read_text(encoding="utf-8")
            projected = {
                line.split("|", 2)[1].strip(): line
                for line in current_table.splitlines()
                if line.startswith("| D10-")
            }
            self.assertEqual(set(ids), set(projected))
            for row in rows:
                key = row["id"]
                line_number, raw = found[key]
                source = row["source"][language]
                fields = [part.strip() for part in raw.split("|")[1:-1]]
                with self.subTest(language=language, case=key):
                    self.assertEqual(str(source_path.relative_to(ROOT)), source["path"])
                    self.assertEqual(sha, source["blob"])
                    self.assertEqual(line_number, source["line"])
                    self.assertEqual(raw, source["raw"])
                    self.assertEqual(fields[1], source["location"])
                    self.assertEqual(fields[2], row["negative"][language])
                    self.assertEqual(fields[3], row["disposition"][language])
                    self.assertEqual(fields[4], row["executionMode"][language])
                    self.assertEqual(fields[5], row["positive"][language])
                    self.assertEqual(fields[6], row["oracle"][language])
                    self.assertIn("| " + fields[1] + " |", projected[key])
                    self.assertIn("| " + fields[5] + " |", projected[key])
                    overlay = row.get("currentOverlay")
                    if overlay and "sourceEraToCurrent" in overlay:
                        changed = overlay["sourceEraToCurrent"].get(language)
                        if changed:
                            self.assertTrue(changed.keys() <= {"location", "positive"})
                            self.assertNotEqual(changed, {k: v for k, v in changed.items() if v == source.get(k)})


if __name__ == "__main__":
    unittest.main()
