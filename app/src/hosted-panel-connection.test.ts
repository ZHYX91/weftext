import assert from "node:assert/strict";
import test from "node:test";
import { connectHostedPanels } from "./hosted-panel-connection";

test("a failed catalog does not suppress an independently authorized panel", async () => {
  const connected: string[] = [];
  const failures = await connectHostedPanels(() => true, [
    { label: "library", connect() { throw new Error("catalog unavailable"); } },
    { label: "import", async connect() { connected.push("import"); } },
    { label: "backup", connect() { connected.push("backup"); } },
  ]);
  assert.deepEqual(connected, ["import", "backup"]);
  assert.deepEqual(failures, [{ label: "library", message: "catalog unavailable" }]);
});

test("revocation during a pending connection prevents every later delivery", async () => {
  let authorized = true, release!: () => void;
  const connected: string[] = [];
  const pending = connectHostedPanels(() => authorized, [
    { label: "library", connect: () => new Promise<void>(resolve => { release = resolve; }) },
    { label: "import", connect() { connected.push("import"); } },
  ]);
  authorized = false; release();
  assert.equal(await pending, null);
  assert.deepEqual(connected, []);
});

test("a late failure from a retired actor cannot report into the replacement connection", async () => {
  let actor = "alice", reject!: (error: Error) => void;
  const pending = connectHostedPanels(() => actor === "alice", [
    { label: "library", connect: () => new Promise<void>((_, fail) => { reject = fail; }) },
    { label: "import", connect() { assert.fail("retired actor continued"); } },
  ]);
  actor = "bob"; reject(new Error("old actor unavailable"));
  assert.equal(await pending, null);
});

test("all independent failures remain visible while a healthy reconnect clears the result", async () => {
  const panels = ["library", "media"].map(label => ({ label, connect() { throw new Error(label); } }));
  assert.deepEqual(await connectHostedPanels(() => true, panels), panels.map(({ label }) => ({ label, message: label })));
  assert.deepEqual(await connectHostedPanels(() => true, [{ label: "library", connect() {} }]), []);
});
