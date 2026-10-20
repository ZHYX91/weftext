import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { EndnoteModuleInstaller, decodeEndnoteModuleState, endnoteModuleReceipt, type EndnoteDistribution, type EndnoteModuleHost, type EndnoteModuleIdentity, type EndnoteModuleReceipt, type EndnoteModuleRequest, type EndnoteModuleResource, type EndnoteModuleState } from "./endnote-module";

const sha = (value: string) => createHash("sha256").update(value).digest("hex");
const original: EndnoteDistribution = { name: "weftext-endnotes.typ", source: "// fixed product module\n#let endnote(body) = body\n", sha256: sha("// fixed product module\n#let endnote(body) = body\n"), version: "1" };
function fixture() {
  let identity: EndnoteModuleIdentity | null = { scope: "local:alice:node-one", owner: "weftext-node://workspace/one", binding: "current-collection-authority", modulePath: "assets/weftext-endnotes.typ" }, stored: unknown = null, module = structuredClone(original), receipt: EndnoteModuleReceipt | null = null, resource: EndnoteModuleResource | null = null, resourceSource = original.source, complete = true, input = true, persistError = false;
  const calls: string[] = [], writes: EndnoteModuleRequest[] = []; let prepareOverride: ((request: EndnoteModuleRequest) => Promise<EndnoteModuleReceipt>) | undefined, commitOverride: (() => Promise<EndnoteModuleReceipt>) | undefined;
  const host: EndnoteModuleHost = {
    identity: () => identity, distribution: async () => { calls.push("distribution"); return structuredClone(module); },
    listResources: async target => { calls.push("list"); return { coverage: complete ? "complete" : "partial", owner: target.owner, resources: resource ? [structuredClone(resource)] : [] }; },
    readResource: async candidate => { calls.push("read"); return { resourceRef: candidate.resourceRef, source: resourceSource, sha256: sha(resourceSource) }; },
    compilerInputAuthorized: async () => { calls.push("compiler-input"); return input; },
    prepareImport: async request => { calls.push("prepare"); assert.ok(stored, "original request is durable before network"); assert.equal((stored as EndnoteModuleState).phase, "preparing"); assert.deepEqual((stored as EndnoteModuleState).request, request); writes.push(structuredClone(request)); if (prepareOverride) return prepareOverride(request); receipt = { operationId: request.operationId, status: "prepared", planId: "plan-original", commitAllowed: true, losses: [] }; return structuredClone(receipt); },
    commitImport: async request => { calls.push("commit"); assert.equal((stored as EndnoteModuleState).phase, "committing"); assert.equal(request.planId, "plan-original"); if (commitOverride) return commitOverride(); receipt = { operationId: request.operationId, status: "committed", planId: "plan-original", commitAllowed: false, resourceRefs: ["weftext-resource://workspace/resource-one"] }; resource = { resourceRef: "weftext-resource://workspace/resource-one", owner: request.identity.owner, modulePath: request.identity.modulePath }; return structuredClone(receipt); },
    importStatus: async request => { calls.push("status"); assert.ok(stored); return structuredClone(receipt ?? { operationId: request.operationId, status: "notRegistered", commitAllowed: false }); },
    settleImport: async request => { calls.push("settle"); return { operationId: request.operationId, status: "cancelled", commitAllowed: false }; },
    load: () => stored, persist: value => { calls.push("persist"); if (persistError) throw new Error("disk full"); stored = structuredClone(value); }, newOperationId: () => { calls.push("allocate"); return "operation-original"; },
  };
  return { host, calls, writes, stored: () => stored as EndnoteModuleState, setIdentity: (value: EndnoteModuleIdentity | null) => { identity = value; }, identity: () => identity!, setModule: (value: EndnoteDistribution) => { module = value; }, setReceipt: (value: EndnoteModuleReceipt | null) => { receipt = value; }, setResource: (value: EndnoteModuleResource | null, source = original.source) => { resource = value; resourceSource = source; }, setComplete: (value: boolean) => { complete = value; }, setInput: (value: boolean) => { input = value; }, failPersist: () => { persistError = true; }, prepare: (fn: typeof prepareOverride) => { prepareOverride = fn; }, commit: (fn: typeof commitOverride) => { commitOverride = fn; } };
}

test("explicit fixed-module installation persists one original request and uses existing resource prepare/commit", async () => {
  const f = fixture(), ready = await new EndnoteModuleInstaller(f.host).install();
  assert.deepEqual(ready, { resourceRef: "weftext-resource://workspace/resource-one", modulePath: "assets/weftext-endnotes.typ", sha256: original.sha256, version: "1", reused: false });
  assert.equal(f.writes.length, 1); assert.equal(f.stored().receipt?.status, "committed"); assert.equal(f.calls.filter(call => call === "allocate").length, 1); assert.equal(f.calls.filter(call => call === "commit").length, 1);
  assert.ok(f.calls.indexOf("persist") < f.calls.indexOf("prepare")); assert.ok(f.calls.indexOf("compiler-input") > f.calls.indexOf("read"));
});

test("only exact bytes of the current owner's selected path are reusable, with separate compiler permission", async () => {
  const f = fixture(); f.setResource({ resourceRef: "existing", owner: f.identity().owner, modulePath: f.identity().modulePath });
  assert.equal((await new EndnoteModuleInstaller(f.host).install()).reused, true); assert.equal(f.calls.includes("prepare"), false); assert.equal(f.calls.includes("allocate"), false);
  f.setInput(false); await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /编译尚无/); assert.equal(f.calls.includes("prepare"), false);
});

test("same-path modified modules, partial absence and foreign ownership never trigger overwrite", async () => {
  const modified = fixture(); modified.setResource({ resourceRef: "existing", owner: modified.identity().owner, modulePath: modified.identity().modulePath }, "// user content");
  await assert.rejects(new EndnoteModuleInstaller(modified.host).install(), /已有不同内容/); assert.equal(modified.calls.includes("prepare"), false);
  const partial = fixture(); partial.setComplete(false); await assert.rejects(new EndnoteModuleInstaller(partial.host).install(), /部分覆盖/); assert.equal(partial.calls.includes("prepare"), false);
  const foreign = fixture(); foreign.setResource({ resourceRef: "other", owner: "other-owner", modulePath: foreign.identity().modulePath }); await assert.rejects(new EndnoteModuleInstaller(foreign.host).install(), /不明确的资源归属/); assert.equal(foreign.calls.includes("read"), false);
});

test("a persistence failure prevents the next resource mutation", async () => {
  const f = fixture(); f.failPersist(); await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /恢复记录不可可靠保存/); assert.equal(f.calls.includes("prepare"), false); assert.equal(f.calls.includes("commit"), false);
});

test("lost preparation recovers the same original operation, without a new producer or ID", async () => {
  const f = fixture(); f.prepare(async request => { f.setReceipt({ operationId: request.operationId, status: "prepared", planId: "plan-original", commitAllowed: true }); throw new Error("lost response"); });
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /lost response/); assert.equal(f.stored().request.operationId, "operation-original");
  const ready = await new EndnoteModuleInstaller(f.host).install(); assert.equal(ready.reused, false); assert.equal(f.writes.length, 1); assert.equal(f.calls.filter(call => call === "allocate").length, 1); assert.equal(f.calls.filter(call => call === "status").length, 1);
});

test("lost commit is status-only recoverable and never reimports the same-name module", async () => {
  const f = fixture(); f.commit(async () => { f.setReceipt({ operationId: "operation-original", status: "committed", planId: "plan-original", commitAllowed: false, resourceRefs: ["original-resource"] }); f.setResource({ resourceRef: "original-resource", owner: f.identity().owner, modulePath: f.identity().modulePath }); throw new Error("lost commit"); });
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /lost commit/);
  const recovery = new EndnoteModuleInstaller(f.host); assert.equal((await recovery.recover())?.status, "committed");
  assert.equal(f.calls.filter(call => call === "prepare").length, 1); assert.equal(f.calls.filter(call => call === "commit").length, 1);
  assert.equal((await recovery.install()).resourceRef, "original-resource"); assert.equal(f.calls.filter(call => call === "commit").length, 1);
});

test("unknown unregistered operation requires explicit retry of its complete original bytes", async () => {
  const f = fixture(); f.prepare(async () => { throw new Error("network before registration"); });
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /network before registration/);
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /按原编号重试/); assert.equal(f.writes.length, 1);
  f.prepare(undefined); await new EndnoteModuleInstaller(f.host).retryOriginal(); assert.equal(f.writes.length, 2); assert.deepEqual(f.writes[0], f.writes[1]); assert.equal(f.calls.filter(call => call === "allocate").length, 1);
});

test("owner switch or installer invalidation after prepare prevents commit, preserving the original record", async () => {
  const f = fixture(); let release!: (receipt: EndnoteModuleReceipt) => void, started!: () => void; const sent = new Promise<void>(resolve => { started = resolve; });
  f.prepare(async () => { started(); return new Promise(resolve => { release = resolve; }); }); const installer = new EndnoteModuleInstaller(f.host), pending = installer.install(); await sent;
  f.setIdentity({ ...f.identity(), owner: "new-owner" }); release({ operationId: "operation-original", status: "prepared", planId: "plan-original", commitAllowed: true });
  await assert.rejects(pending, /归属、授权/); assert.equal(f.calls.includes("commit"), false); assert.equal(f.stored().request.identity.owner, "weftext-node://workspace/one");
});

test("changed distribution invalidates unconfirmed installation without replacing original bytes", async () => {
  const f = fixture(); f.prepare(async request => { f.setReceipt({ operationId: request.operationId, status: "prepared", planId: "plan-original", commitAllowed: true }); throw new Error("lost"); });
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /lost/); const updatedSource = original.source + "// version 2\n"; f.setModule({ ...original, source: updatedSource, sha256: sha(updatedSource), version: "2" });
  await assert.rejects(new EndnoteModuleInstaller(f.host).install(), /模块版本已变化/); assert.equal(f.calls.includes("commit"), false); assert.equal(f.stored().request.module.source, original.source);
});

test("a distinct explicit new installation requires fresh server cancellation of the old operation", async () => {
  const f = fixture(); f.prepare(async () => { throw new Error("unknown"); }); const installer = new EndnoteModuleInstaller(f.host);
  await assert.rejects(installer.install(), /unknown/); await assert.rejects(installer.restartAfterCancelled(), /尚未明确取消/);
  assert.equal(f.calls.filter(call => call === "allocate").length, 1);
  f.setReceipt({ operationId: "operation-original", status: "cancelled", commitAllowed: false }); f.prepare(undefined); f.host.newOperationId = () => "explicit-new-operation";
  await installer.restartAfterCancelled(); assert.equal(f.stored().request.operationId, "explicit-new-operation"); assert.equal(f.writes.length, 2); assert.notEqual(f.writes[0].operationId, f.writes[1].operationId);
});

test("a replaced same-path resource cannot claim the original committed receipt", async () => {
  const f = fixture(); const installer = new EndnoteModuleInstaller(f.host); await installer.install(); f.setResource({ resourceRef: "replacement-resource", owner: f.identity().owner, modulePath: f.identity().modulePath });
  await assert.rejects(installer.install(), /不是原安装回执的资源/); assert.equal(f.calls.filter(call => call === "prepare").length, 1);
});

test("malformed caches and wrong-plan receipts retain original responsibility", () => {
  assert.throws(() => decodeEndnoteModuleState({ format: "unknown" }), /原安装记录无法核对/);
  const request: EndnoteModuleRequest = { operationId: "one", identity: { scope: "actor", owner: "owner", binding: "binding", modulePath: "weftext-endnotes.typ" }, module: original };
  assert.throws(() => endnoteModuleReceipt(request, { operationId: "other", status: "prepared", planId: "p", commitAllowed: true }), /不对应原操作/);
  assert.throws(() => endnoteModuleReceipt(request, { operationId: "one", status: "prepared", planId: "new", commitAllowed: true }, { operationId: "one", status: "prepared", planId: "old", commitAllowed: true }), /原计划或提交事实/);
});
