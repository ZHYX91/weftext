"""Pure ownership guards only; never starts simctl, Xcode or any OS process."""
from pathlib import Path
import importlib.util
import unittest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("weftext_apple_entry", ROOT / "scripts" / "apple.py")
ENTRY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ENTRY)


class SimulatorOwnershipTests(unittest.TestCase):
    UDID = "A9642E97-FC21-4E4B-B384-DA49AEAC78BF"
    NAME = "Weftext-39b8e279-5736-4da5-b7b4-b8511781b934"

    def device(self, **changes):
        value = {"udid": self.UDID, "name": self.NAME, "deviceTypeIdentifier": ENTRY.SIMULATOR_DEVICE_TYPE,
                 "isAvailable": True, "state": "Shutdown"}
        value.update(changes)
        return value

    def inventory(self, device=None, runtime=None):
        return {"devices": {runtime or ENTRY.SIMULATOR_RUNTIME: [device or self.device()]}}

    def test_actual_original_tuple_required_before_any_owned_action(self):
        self.assertEqual(ENTRY.owned_device(self.inventory(), self.UDID, self.NAME)["state"], "Shutdown")
        for changed in [{"name": "other"}, {"deviceTypeIdentifier": "other"}, {"isAvailable": False},
                        {"deviceTypeIdentifier": None}]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                ENTRY.owned_device(self.inventory(self.device(**changed)), self.UDID, self.NAME)
        with self.assertRaises(ValueError):
            ENTRY.owned_device(self.inventory(runtime="foreign-runtime"), self.UDID, self.NAME)

    def test_duplicate_udid_cannot_be_hidden_in_another_runtime(self):
        inventory = self.inventory()
        inventory["devices"]["foreign-runtime"] = [self.device(name="other")]
        with self.assertRaises(ValueError):
            ENTRY.owned_device(inventory, self.UDID, self.NAME)

    def test_ambiguous_create_reconciles_only_new_exact_original_tuple(self):
        before = {"devices": {ENTRY.SIMULATOR_RUNTIME: []}}
        self.assertEqual(ENTRY.reconcile_created_device(self.inventory(), before, self.NAME), self.UDID)
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(self.inventory(), self.inventory(), self.NAME)
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(before, before, self.NAME)
        ambiguous = self.inventory()
        ambiguous["devices"][ENTRY.SIMULATOR_RUNTIME].append(self.device(udid="F9A74953-0178-493A-A59C-BB6C58A23D28"))
        with self.assertRaises(ValueError):
            ENTRY.reconcile_created_device(ambiguous, before, self.NAME)

    def test_uuid_aliases_are_not_os_identity_receipts(self):
        self.assertEqual(ENTRY.simulator_udid(self.UDID.lower()), self.UDID)
        for alias in ["{" + self.UDID + "}", self.UDID.replace("-", ""), " " + self.UDID, None]:
            with self.subTest(alias=alias), self.assertRaises((ValueError, TypeError)):
                ENTRY.simulator_udid(alias)

    def test_duplicate_inventory_keys_are_not_silently_rebound(self):
        with self.assertRaises(ValueError):
            ENTRY.strict_inventory('{"devices":{},"devices":{"different":[]}}')


if __name__ == "__main__":
    unittest.main()
