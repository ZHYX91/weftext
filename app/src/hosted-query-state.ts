import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { QueryResult, QueryRow } from "./query-panel";
import { decodeHostedQueryResponse } from "./query-result";
import { parseQueryJson, parseQueryPlan, type QueryPlan } from "./query-panel";
export type HostedQueryRow = QueryRow & {
    resource?: string;
    canOpen?: boolean;
};
export type HostedQueryResult = QueryResult & {
    rows: HostedQueryRow[];
    columns: string[];
    queryId: string;
};
export interface HostedQueryResponse {
    scope: "authorizedProjection";
    readOnly: true;
    queryId: string;
    columns: string[];
    result: QueryResult & {
        rows: HostedQueryRow[];
    };
}
export interface HostedProviderDescriptor {
    id: string;
    fingerprint: string;
    implementation: {
        id: string;
        abi: number;
    };
    data: {
        kind: string;
        key?: string;
        version?: string;
    };
    limits: {
        maxSteps: number;
        maxBytes: number;
        maxExpansion: number;
    };
    queryNames: string[];
}
export interface HostedSeriesPolicyDescriptor {
    id: string;
    fingerprint: string;
    key: string;
    kinds: string[];
    rule: string;
    calendar: string | null;
    zone: string | null;
    status: "available" | "unavailable";
}
export interface HostedQueryDependencies {
    providers: Record<string, string>;
    seriesPolicies: Record<string, string>;
}
export interface HostedDirectoryDescriptor {
    id: string;
    title: string;
    reference: string;
    kind: "node" | "directory";
    lifecycle?: "live" | "trash";
    status: "available" | "unavailable";
}
export interface HostedScopeCatalog {
    directories?: HostedDirectoryDescriptor[];
    trash?: boolean;
    scopeCoverage?: "complete" | "partial";
}
export function hostedScopeDraft(value: unknown): QueryPlan["scope"] | undefined {
    if (value === undefined)
        return undefined;
    if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).some(key => !["directory", "descendants", "lifecycle"].includes(key)))
        throw new UiError(uiMessage("hostedQueryState.scopeRecoveryInvalid"));
    const scope = value as QueryPlan["scope"];
    if (typeof scope.descendants !== "boolean" || !["live", "trash"].includes(scope.lifecycle) || scope.directory !== undefined && typeof scope.directory !== "string")
        throw new UiError(uiMessage("hostedQueryState.scopeRecoveryInvalid"));
    return { ...scope };
}
export function applyHostedScope(source: string, scope: QueryPlan["scope"]): string {
    const plan = parseQueryPlan(source);
    if ((plan.advanced?.call || plan.advanced?.unionAll) && (scope.directory || !scope.descendants || scope.lifecycle !== "live"))
        throw new UiError(uiMessage("hostedQueryState.outerScopeMustStayDefault"));
    const raw = parseQueryJson(source) as Record<string, unknown>, changed = { ...(raw.scope as object ?? {}), descendants: scope.descendants, lifecycle: scope.lifecycle } as QueryPlan["scope"];
    if (scope.directory)
        changed.directory = scope.directory;
    else
        delete changed.directory;
    return JSON.stringify({ ...raw, scope: changed }, null, 2);
}
export function exactHostedScope(scope: QueryPlan["scope"], catalog: HostedScopeCatalog): void {
    if (scope.lifecycle === "trash" && catalog.trash !== true)
        throw new UiError(uiMessage("hostedQueryState.recycleScopeReadNotPermitted"));
    if (!scope.directory)
        return;
    const directory = catalog.directories?.find(item => item.id === scope.directory || item.reference === scope.directory);
    if (!directory || directory.status !== "available")
        throw new UiError(displayMessage("hostedQueryState.originalDirectoryUnavailable", { p0: uiDisplayParameter(scope.directory) }));
    if ((directory.lifecycle ?? "live") !== scope.lifecycle)
        throw new UiError(uiMessage("hostedQueryState.directoryLifecycleMismatch"));
}
export function hostedDependencyMap(value: unknown): Record<string, string> {
    if (value === undefined)
        return {};
    if (!value || typeof value !== "object" || Array.isArray(value) || Object.entries(value).some(([id, fingerprint]) => !id || typeof fingerprint !== "string" || !/^[0-9a-f]{64}$/.test(fingerprint)))
        throw new UiError(uiMessage("hostedQueryState.providerRecoveryFingerprintMissing"));
    return Object.fromEntries(Object.entries(value));
}
export function exactHostedDependencies(selection: HostedQueryDependencies, catalog: {
    providers?: Pick<HostedProviderDescriptor, "id" | "fingerprint">[];
    seriesPolicies?: Pick<HostedSeriesPolicyDescriptor, "id" | "fingerprint">[];
}): HostedQueryDependencies {
    const result = { providers: hostedDependencyMap(selection.providers), seriesPolicies: hostedDependencyMap(selection.seriesPolicies) };
    for (const kind of ["providers", "seriesPolicies"] as const)
        for (const [id, fingerprint] of Object.entries(result[kind])) {
            const current = catalog[kind]?.find(item => item.id === id);
            if (!current || current.fingerprint !== fingerprint)
                throw new UiError(displayMessage("hostedQueryState.originalChoiceUnavailable", { p0: uiDisplayParameter(kind === "providers" ? " Provider" : uiMessage("hostedQueryState.seriesPolicy")), p1: uiDisplayParameter(id) }));
        }
    return result;
}
/** A frozen result has one exact column order, including complete empty results. */
export function decodeHostedQuery(response: HostedQueryResponse): HostedQueryResult {
    return decodeHostedQueryResponse(response);
}
/** Input edits, ABA plan changes and actor reset independently invalidate delivery. */
export class HostedQueryGate {
    private epoch = 0;
    private generation = 0;
    private revision = 0;
    changed(): void { this.revision++; }
    clear(): void { this.epoch++; this.generation++; this.revision++; }
    begin(input: string) { return { epoch: this.epoch, generation: ++this.generation, revision: this.revision, input }; }
    current(token: ReturnType<HostedQueryGate["begin"]>, input: string): boolean { return token.epoch === this.epoch && token.generation === this.generation && token.revision === this.revision && token.input === input; }
}
