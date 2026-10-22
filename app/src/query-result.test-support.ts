import { message } from "./ui-locale";
import type { QueryResult } from "./query-panel";
/** Actual Data response facts; tests add endpoint-specific application envelopes explicitly. */
export function queryDataResult(overrides:Partial<QueryResult>={}) {
    return {cutId:"cut",schemaRevision:"schema",rows:[],coverage:"complete" as const,freshness:"current" as const,diagnostics:[],totalKnown:0,limited:false,aggregateExact:true,runtime:"test-cel",terminalSchema:{},...overrides};
}
export function localQueryResult(overrides:Record<string,unknown>={}) {
    return {...queryDataResult(),engineDiagnostics:[],productErrors:[],columns:[],snapshotMode:"admittedFrozen",snapshotLabel:message("queryStatus.frozenSnapshotFallback"),externalPending:false,rawScope:{complete:true,pendingCount:0},membershipIds:[],scopeCuts:{},inputVersions:[],access:{audience:"authenticatedLocalSession",scope:"admittedNodesAtSelectedLifecycle",bodyProduct:false,titleProduct:false,headingsProduct:false,resultGrantsWriteAccess:false},queryId:"original",...overrides};
}
