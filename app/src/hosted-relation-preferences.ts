import type { HostedLinksContext, HostedLinksProfile } from "./hosted-knowledge-links-state";
import { issueRuleKey, type PersonalRule } from "./relationship-diagnostics-state";
import { parseStrictJson } from "./strict-json";

export interface HostedRelationScope { deployment: string; actor: string; workspace: string }
type RuleStorage = Pick<Storage, "getItem" | "setItem">;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const MAX_RULES = 1000, MAX_BYTES = 128 * 1024;
const fail = (): never => { throw new Error("Personal relationship preferences are unavailable or have changed."); };

/** The verified server read identifies the actual library; a configured resource name does not. */
export function hostedRelationScope(context: HostedLinksContext, profile: HostedLinksProfile): HostedRelationScope {
    const sources = profile.diagnostics?.nodes.filter(item => item.node?.resource === context.resource) ?? [];
    const source = sources.length === 1 ? sources[0].node.nodeRef : undefined;
    if (!context.deployment || !context.actor || !source || !UUID.test(source.workspaceId) || !UUID.test(source.nodeId)
        || profile.graph?.root !== `${source.workspaceId}/${source.nodeId}`)
        return fail();
    return { deployment: context.deployment, actor: context.actor, workspace: source.workspaceId };
}

export function hostedRelationPreferenceKey(scope: HostedRelationScope): string {
    if (!scope.deployment || !scope.actor || !UUID.test(scope.workspace)) return fail();
    return `weftext-hosted-relations-v1:${JSON.stringify([scope.deployment, scope.actor, scope.workspace])}`;
}

function validateRules(value: unknown): PersonalRule[] {
    if (!Array.isArray(value) || value.length > MAX_RULES) return fail();
    return value.map(rule => {
        if (!rule || typeof rule !== "object" || Array.isArray(rule)
            || Object.keys(rule).some(key => !["kind", "value", "source"].includes(key))
            || !["issue", "expected", "term", "target"].includes(rule.kind) || typeof rule.value !== "string"
            || rule.source !== undefined && typeof rule.source !== "string") return fail();
        return { kind: rule.kind, value: rule.value, ...(rule.source === undefined ? {} : { source: rule.source }) } as PersonalRule;
    });
}
function decode(raw: string | null): PersonalRule[] {
    if (raw === null) return [];
    if (new TextEncoder().encode(raw).length > MAX_BYTES) return fail();
    const record = parseStrictJson(raw);
    if (!record || typeof record !== "object" || Array.isArray(record)
        || Object.keys(record).sort().join(",") !== "rules,version"
        || (record as { version: unknown }).version !== 1) return fail();
    return validateRules((record as { rules: unknown }).rules);
}

/** Reapply only against the fresh authorized result. Dormant choices are never disclosed as a cache. */
export function hostedRelationRuleVisibility(profile: HostedLinksProfile): (rule: PersonalRule) => boolean {
    const root = profile.graph?.root;
    const nodes = new Set(profile.diagnostics?.nodes.map(item => `${item.node.nodeRef.workspaceId}/${item.node.nodeRef.nodeId}`));
    const issues = new Set(profile.diagnostics?.issues.map(item => issueRuleKey({
        source: { key: `${item.source.nodeRef.workspaceId}/${item.source.nodeRef.nodeId}`, title: item.source.title, reference: item.source.reference },
        address: item.occurrence.target, status: item.resolution.status,
    })));
    for (const item of profile.diagnostics?.propertyIssues ?? []) issues.add(issueRuleKey({
        source: { key: `${item.source.nodeRef.workspaceId}/${item.source.nodeRef.nodeId}`, title: item.source.title, reference: item.source.reference },
        address: item.address, status: item.resolution.status,
    }));
    const terms = new Set<string>(), targets = new Set<string>();
    for (const fragment of profile.mentions.fragments) for (const hit of fragment.hits) {
        for (const candidate of hit.candidates ?? hit.targets.map(target => ({ target, term: hit.terms[0] }))) {
            terms.add(candidate.term);
            targets.add(`${candidate.target.nodeRef.workspaceId}/${candidate.target.nodeRef.nodeId}`);
        }
    }
    return rule => (!rule.source || rule.source === root) && (rule.kind === "issue" ? issues.has(rule.value)
        : rule.kind === "expected" ? nodes.has(rule.value)
            : rule.kind === "term" ? terms.has(rule.value) : targets.has(rule.value));
}

/** Stores user-selected rules only, with no diagnostic results, source snapshots or read receipts. */
export class HostedRelationPreferences {
    private original: string | null;
    private allRules: PersonalRule[];
    private readonly key: string;
    private readonly visible: (rule: PersonalRule) => boolean;
    readonly rules: PersonalRule[];
    constructor(private readonly storage: () => RuleStorage, scope: HostedRelationScope, profile: HostedLinksProfile,
        private readonly current: () => boolean) {
        this.key = hostedRelationPreferenceKey(scope);
        this.original = storage().getItem(this.key);
        this.allRules = decode(this.original);
        this.visible = hostedRelationRuleVisibility(profile);
        this.rules = this.allRules.filter(this.visible);
    }
    save(next: readonly PersonalRule[]): void {
        if (!this.current()) return fail();
        const selected = validateRules(next);
        if (selected.some(rule => !this.visible(rule))) return fail();
        // A narrower authorized read must not erase another note's dormant preferences.
        const merged = [...this.allRules.filter(rule => !this.visible(rule)), ...selected];
        validateRules(merged);
        const raw = JSON.stringify({ version: 1, rules: merged });
        if (new TextEncoder().encode(raw).length > MAX_BYTES) return fail();
        const storage = this.storage();
        if (storage.getItem(this.key) !== this.original || !this.current()) return fail();
        storage.setItem(this.key, raw);
        if (storage.getItem(this.key) !== raw) return fail();
        this.original = raw;
        this.allRules = merged;
    }
}
