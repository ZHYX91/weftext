import test from "node:test";
import assert from "node:assert/strict";
import { HostedRelationPreferences, hostedRelationPreferenceKey, hostedRelationRuleVisibility, hostedRelationScope } from "./hosted-relation-preferences";
import type { HostedLinksContext, HostedLinksNode, HostedLinksProfile } from "./hosted-knowledge-links-state";
import type { PersonalRule } from "./relationship-diagnostics-state";

const workspace = "10000000-0000-4000-8000-000000000001", otherWorkspace = "20000000-0000-4000-8000-000000000001";
const sourceId = "10000000-0000-4000-8000-000000000002", targetId = "10000000-0000-4000-8000-000000000003";
const context = { deployment: "server", actor: "alice", resource: "Shared", generation: 1 } as HostedLinksContext;
const node = (nodeId: string, resource: string, library = workspace): HostedLinksNode => ({ resource, title: `PROTECTED_TITLE_${nodeId}`, nodeRef: { workspaceId: library, nodeId }, reference: `weftext-node://${library}/${nodeId}`, canOpen: true });
function profile(library = workspace): HostedLinksProfile {
    const source = node(sourceId, "Shared", library), target = node(targetId, "Other", library), root = `${library}/${sourceId}`;
    return {
        graph: { root, nodes: [], edges: [], coverage: "complete", scope: "authorizedNativeSourcesAndTypedReferences", generation: "generation", truncated: false },
        diagnostics: { coverage: "complete", scannedNodes: 2, nodes: [source, target].map(node => ({ node, incoming: 0, outgoing: 0, invalidOutgoing: 0, state: "isolated" })),
            issues: [{ source, occurrence: { target: "../Missing.typ", from: 0, to: 10, utf16From: 0, utf16To: 10, kind: "link" }, raw: "PROTECTED_LINK_RESULT", resolution: { status: "missing", canOpen: false }, editable: true }] },
        mentions: { scope: "nativeTitlesAndAuthorizedNameFields", coverage: "complete", fragments: [{ id: "f", text: "PROTECTED_BODY_TEXT", coverage: "complete", hits: [{ terms: ["chosen term"], candidates: [{ target, term: "chosen term", provenance: "title" }], targets: [target], writable: true, startScalar: 0, endScalar: 11, startUtf16: 0, endUtf16: 11, component: 0 }] }] },
        basis: { actor: "alice", actorEpoch: 1, deployment: "server", resource: "Shared", sessionId: "session", serial: 1, sourceHash: "b".repeat(64), snapshot: "a".repeat(64) },
        scope: "configuredAuthorizedManagedSources", writable: true, fileConversion: false,
        links: { coverage: "complete", outgoing: [], incomingId: [], incomingPath: [] }, renderedLinks: [], propertyReferences: { scope: "explicitAuthorizedFields", coverage: "complete", items: [] },
    };
}
class MemoryStorage {
    values = new Map<string, string>();
    refused = false;
    getItem(key: string): string | null { return this.values.get(key) ?? null; }
    setItem(key: string, value: string): void { if (this.refused) throw new Error("QuotaExceededError"); this.values.set(key, value); }
}
const chosen: PersonalRule = { kind: "term", value: "chosen term", source: `${workspace}/${sourceId}` };

test("hosted rule choices survive a new controller without persisting source, titles, result caches or credentials", () => {
    const storage = new MemoryStorage(), data = profile(), scope = hostedRelationScope(context, data);
    const original = new HostedRelationPreferences(() => storage, scope, data, () => true);
    original.save([chosen]);
    const reopened = new HostedRelationPreferences(() => storage, scope, profile(), () => true);
    assert.deepEqual(reopened.rules, [chosen]);
    const raw = [...storage.values.values()][0];
    assert.deepEqual(JSON.parse(raw), { version: 1, rules: [chosen] });
    for (const protectedValue of ["PROTECTED_BODY_TEXT", "PROTECTED_TITLE", "PROTECTED_LINK_RESULT", "sourceHash", "snapshot", "credential", "generation"]) assert.ok(!raw.includes(protectedValue));
});

test("real deployment, actor and confirmed workspace partition preferences; configured aliases and delimiter collisions do not", () => {
    const storage = new MemoryStorage(), scope = hostedRelationScope(context, profile());
    new HostedRelationPreferences(() => storage, scope, profile(), () => true).save([chosen]);
    for (const other of [{ ...scope, actor: "bob" }, { ...scope, deployment: "other" }, { ...scope, workspace: otherWorkspace }]) {
        assert.deepEqual(new HostedRelationPreferences(() => storage, other, profile(other.workspace), () => true).rules, []);
    }
    assert.notEqual(hostedRelationPreferenceKey({ ...scope, deployment: "x:y", actor: "z" }), hostedRelationPreferenceKey({ ...scope, deployment: "x", actor: "y:z" }));
    const changedAlias = profile(); changedAlias.diagnostics!.nodes[0].node.resource = "RenamedAlias";
    assert.deepEqual(hostedRelationScope({ ...context, resource: "RenamedAlias" }, changedAlias), scope);
    for (const change of [() => { const data = profile(); data.graph!.root = `${otherWorkspace}/${sourceId}`; return data; }, () => { const data = profile(); data.diagnostics!.nodes = []; return data; }, () => { const data = profile(); data.diagnostics!.nodes.push(data.diagnostics!.nodes[0]); return data; }]) assert.throws(() => hostedRelationScope(context, change()));
});

test("revocation hides dormant object/address/term rules and Undo cannot resurrect them; later saves preserve those preferences privately", () => {
    const storage = new MemoryStorage(), data = profile(), scope = hostedRelationScope(context, data);
    const all: PersonalRule[] = [chosen, { kind: "target", value: `${workspace}/${targetId}` }, { kind: "expected", value: `${workspace}/${targetId}` }, { kind: "issue", value: JSON.stringify([`${workspace}/${sourceId}`, "../Missing.typ"]) }];
    new HostedRelationPreferences(() => storage, scope, data, () => true).save(all);
    const revoked = profile(); revoked.diagnostics!.nodes.splice(1); revoked.diagnostics!.issues = []; revoked.mentions.fragments = [];
    const narrowed = new HostedRelationPreferences(() => storage, scope, revoked, () => true);
    assert.deepEqual(narrowed.rules, []);
    for (const rule of all) assert.equal(hostedRelationRuleVisibility(revoked)(rule), false);
    assert.throws(() => narrowed.save([chosen]));
    narrowed.save([{ kind: "expected", value: `${workspace}/${sourceId}` }]);
    const restored = new HostedRelationPreferences(() => storage, scope, data, () => true);
    assert.deepEqual(restored.rules, [...all, { kind: "expected", value: `${workspace}/${sourceId}` }]);
});

test("scope mismatch, concurrent changes and quota failures leave the original choices unchanged; confirmed Undo is persisted", () => {
    const storage = new MemoryStorage(), scope = hostedRelationScope(context, profile());
    let current = true;
    const preferences = new HostedRelationPreferences(() => storage, scope, profile(), () => current);
    storage.refused = true; assert.throws(() => preferences.save([chosen])); assert.equal(storage.values.size, 0);
    storage.refused = false; preferences.save([chosen]);
    const first = storage.getItem(hostedRelationPreferenceKey(scope));
    current = false; assert.throws(() => preferences.save([])); assert.equal(storage.getItem(hostedRelationPreferenceKey(scope)), first);
    current = true; preferences.save([]); assert.deepEqual(new HostedRelationPreferences(() => storage, scope, profile(), () => true).rules, []);
    storage.setItem(hostedRelationPreferenceKey(scope), JSON.stringify({ version: 1, rules: [chosen] }));
    assert.throws(() => preferences.save([])); assert.deepEqual(new HostedRelationPreferences(() => storage, scope, profile(), () => true).rules, [chosen]);
});

test("corrupt, oversized, unknown and duplicate-member records remain untouched and cannot be silently repaired as an empty preference list", () => {
    const storage = new MemoryStorage(), scope = hostedRelationScope(context, profile()), key = hostedRelationPreferenceKey(scope);
    for (const raw of ['{', '{"version":1,"version":1,"rules":[]}', JSON.stringify({ version: 2, rules: [] }), JSON.stringify({ version: 1, rules: [{ ...chosen, sourceHash: "hidden" }] }), " ".repeat(128 * 1024 + 1), JSON.stringify({ version: 1, rules: Array(1001).fill(chosen) })]) {
        storage.setItem(key, raw);
        assert.throws(() => new HostedRelationPreferences(() => storage, scope, profile(), () => true));
        assert.equal(storage.getItem(key), raw);
    }
});
