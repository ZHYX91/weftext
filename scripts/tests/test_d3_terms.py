import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("terms", ROOT / "scripts/check_d3_terms.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SOURCE = '''### weftext.term.sample
- 定义：head、
  tail obligation。
- owner/layer：D3；
  D6 obligations。
- firstFreeze：frozen:D2。
- migration/deletion：retain
- 受控集合：
~~~json
{"ownedNames":{"wireApiNames":["Sample"]},"semanticNonaliases":["one","two"]}
~~~
'''


class ProjectionTests(unittest.TestCase):
    def machine(self):
        return {"concept_count": 1, "concepts": MODULE.project(SOURCE)}

    def test_complete(self):
        self.assertEqual(MODULE.compare(self.machine(), SOURCE), [])

    def test_physical_line_invariance(self):
        reflowed = SOURCE.replace("head、\n  tail", "head、 tail").replace("D3；\n  D6", "D3； D6")
        self.assertEqual(MODULE.compare(self.machine(), reflowed), [])

    def test_truncated_definition(self):
        machine = self.machine()
        machine["concepts"][0]["currentDefinition"] = "head、"
        self.assertTrue(MODULE.compare(machine, SOURCE))

    def test_truncated_owner(self):
        machine = self.machine()
        machine["concepts"][0]["currentOwnerLayer"] = "D3；"
        self.assertTrue(MODULE.compare(machine, SOURCE))

    def test_sets_and_freeze(self):
        changes = (
            ("firstFreeze", "frozen:wrong"),
            ("semanticNonaliases", ["two", "one"]),
            ("ownedNames", {"wireApiNames": ["Other"]}),
        )
        for key, value in changes:
            with self.subTest(key=key):
                machine = self.machine()
                machine["concepts"][0][key] = value
                self.assertTrue(MODULE.compare(machine, SOURCE))

    def test_duplicate_concept(self):
        with self.assertRaises(ValueError):
            MODULE.project(SOURCE + SOURCE)

    def test_actual_projection(self):
        base = ROOT / "docs/design/a2-final-design"
        machine = json.loads((base / "D3-TERMS.json").read_text(encoding="utf-8-sig"))
        self.assertEqual(machine["concept_count"], 42)
        self.assertEqual(MODULE.validate(ROOT), [])


if __name__ == "__main__":
    unittest.main()
