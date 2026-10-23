import { decodeCell, type Cell } from "./data-projection";
import { decodeApiErrorBody, apiErrorUiText, type ApiErrorBody } from "./api-error";
import { decodeCoreDescription, decodeErrorDescription, decodeEngineDiagnostic, coreDescriptionUiText, engineDiagnosticUiText, exactKeys, type CoreDescription, type EngineDiagnostic } from "./ui-diagnostics";
import { joinUiText, message, parseUiMessage, UiError, type UiText } from "./ui-locale";
import type { QueryResult, QueryRow } from "./query-panel";

export interface QueryEngineGroup {entityKey:string;diagnostics:readonly EngineDiagnostic[]}
export interface QueryProductError {entityKey:string;error:ApiErrorBody}
const object=(value:unknown):Record<string,unknown>|undefined=>value && typeof value==="object" && !Array.isArray(value) ? value as Record<string,unknown> : undefined;
const strings=(value:unknown):value is string[]=>Array.isArray(value) && value.every(item=>typeof item==="string");
const nonnegative=(value:unknown):value is number=>typeof value==="number" && Number.isSafeInteger(value) && value>=0;
const invalid=():never=>{throw new UiError(message("error.invalidResponse"));};
const BASE=["cutId","schemaRevision","rows","coverage","freshness","diagnostics","totalKnown","limited","aggregateExact","runtime","terminalSchema"];
const LOCAL=["engineDiagnostics","productErrors","columns","snapshotMode","snapshotLabel","externalPending","rawScope","membershipIds","scopeCuts","inputVersions","access"];

function queryType(value:unknown,depth=0):boolean {
    const item=object(value); if(!item || depth>256) return false;
    if(item.kind==="any") return exactKeys(item,["kind"]);
    if(!exactKeys(item,["kind","value"])) return false;
    if(item.kind==="scalar") return typeof item.value==="string";
    if(item.kind==="list" || item.kind==="map") return queryType(item.value,depth+1);
    if(item.kind==="union") return Array.isArray(item.value) && item.value.every(value=>queryType(value,depth+1));
    const members=object(item.value); return item.kind==="record" && !!members && Object.values(members).every(value=>queryType(value,depth+1));
}

function rows(value:unknown,hosted:boolean): QueryRow[]|undefined {
    if(!Array.isArray(value)) return;
    const result:QueryRow[]=[];
    for(const candidate of value) {
        const row=object(candidate); if(!row) return;
        const projected=hosted && row.entityKey!==null;
        if(!exactKeys(row,["key","entityKey","path","revision","cells","readOnly",...(projected?["resource","canOpen"]:[])],["lifecycle"]) || typeof row.key!=="string" || row.entityKey!==null && typeof row.entityKey!=="string" || row.path!==null && typeof row.path!=="string" || row.revision!==null && typeof row.revision!=="string" || typeof row.readOnly!=="boolean" || Object.hasOwn(row,"lifecycle") && typeof row.lifecycle!=="string" || projected && (typeof row.resource!=="string" || typeof row.canOpen!=="boolean")) return;
        const input=object(row.cells); if(!input) return;
        const cells:[string,Cell][]=[];
        for(const [name,value] of Object.entries(input)) {const cell=decodeCell(value);if(!cell)return;cells.push([name,cell]);}
        result.push({...row,cells:Object.fromEntries(cells)} as unknown as QueryRow);
    }
    return result;
}

function base(item:Record<string,unknown>,hosted:boolean): QueryResult {
    const decodedRows=rows(item.rows,hosted),schema=object(item.terminalSchema);
    if(!decodedRows || !schema || !Object.values(schema).every(value=>queryType(value)) || typeof item.cutId!=="string" || typeof item.schemaRevision!=="string" || typeof item.runtime!=="string" || !["complete","partial"].includes(String(item.coverage)) || !["current","stale"].includes(String(item.freshness)) || !nonnegative(item.totalKnown) || item.totalKnown<decodedRows.length || typeof item.limited!=="boolean" || typeof item.aggregateExact!=="boolean" || !Array.isArray(item.diagnostics)) return invalid();
    const diagnostics:CoreDescription[]=[];
    for(const value of item.diagnostics) {const description=hosted?decodeErrorDescription(value):decodeCoreDescription(value); if(!description)return invalid(); diagnostics.push(description);}
    return {...item,rows:decodedRows,diagnostics} as unknown as QueryResult;
}

export function decodeLocalQueryResult(value:unknown): QueryResult {
    const item=object(value); if(!item || !exactKeys(item,[...BASE,...LOCAL],["queryId","exportUnavailable"]) || Object.hasOwn(item,"queryId")===Object.hasOwn(item,"exportUnavailable")) return invalid();
    const result=base(item,false),snapshotLabel=parseUiMessage(item.snapshotLabel),exportUnavailable=Object.hasOwn(item,"exportUnavailable")?parseUiMessage(item.exportUnavailable):undefined;
    const rawScope=object(item.rawScope),scopeCuts=object(item.scopeCuts),access=object(item.access);
    if(!snapshotLabel || !strings(item.columns) || new Set(item.columns).size!==item.columns.length || item.snapshotMode!=="admittedFrozen" || typeof item.externalPending!=="boolean" || !rawScope || !exactKeys(rawScope,["complete","pendingCount"]) || typeof rawScope.complete!=="boolean" || !nonnegative(rawScope.pendingCount) || !strings(item.membershipIds) || !scopeCuts || Object.values(scopeCuts).some(value=>typeof value!=="string") || !Array.isArray(item.inputVersions) || !access || !exactKeys(access,["audience","scope","bodyProduct","titleProduct","headingsProduct","resultGrantsWriteAccess"]) || access.audience!=="authenticatedLocalSession" || access.scope!=="admittedNodesAtSelectedLifecycle" || [access.bodyProduct,access.titleProduct,access.headingsProduct].some(value=>typeof value!=="boolean") || access.resultGrantsWriteAccess!==false || Object.hasOwn(item,"queryId") && (typeof item.queryId!=="string" || !item.queryId) || Object.hasOwn(item,"exportUnavailable") && !exportUnavailable || !Array.isArray(item.engineDiagnostics) || !Array.isArray(item.productErrors)) return invalid();
    const engineDiagnostics:QueryEngineGroup[]=[],productErrors:QueryProductError[]=[];
    for(const value of item.engineDiagnostics) {
        const group=object(value);if(!group || !exactKeys(group,["entityKey","diagnostics"]) || typeof group.entityKey!=="string" || !Array.isArray(group.diagnostics))return invalid();
        const diagnostics:EngineDiagnostic[]=[];for(const value of group.diagnostics){const diagnostic=decodeEngineDiagnostic(value);if(!diagnostic)return invalid();diagnostics.push(diagnostic);}engineDiagnostics.push({entityKey:group.entityKey,diagnostics});
    }
    for(const value of item.productErrors) {const group=object(value),error=group && decodeApiErrorBody(group.error);if(!group || !exactKeys(group,["entityKey","error"]) || typeof group.entityKey!=="string" || !error)return invalid();productErrors.push({entityKey:group.entityKey,error});}
    return {...result,columns:[...item.columns],snapshotLabel,engineDiagnostics,productErrors,...(exportUnavailable?{exportUnavailable}:{})};
}

export function decodeHostedQueryResult(value:unknown,embedded=false):QueryResult {
    const item=object(value);if(!item || !exactKeys(item,[...BASE,...(embedded?["queryId","columns"]:[])]))return invalid();
    const result=base(item,true);
    if(embedded && (typeof item.queryId!=="string" || !item.queryId || !strings(item.columns) || new Set(item.columns).size!==item.columns.length))return invalid();
    return result;
}

export function decodeHostedQueryResponse(value:unknown):QueryResult & {columns:string[];queryId:string} {
    const item=object(value);if(!item || !exactKeys(item,["result","queryId","columns","scope","readOnly"]) || typeof item.queryId!=="string" || !item.queryId || !strings(item.columns) || new Set(item.columns).size!==item.columns.length || item.scope!=="authorizedProjection" || item.readOnly!==true)return invalid();
    const result=decodeHostedQueryResult(item.result);
    if(result.rows.some(row=>Object.keys(row.cells).some(name=>!(item.columns as string[]).includes(name))))return invalid();
    return {...result,columns:[...item.columns],queryId:item.queryId};
}

/** Each explanation retains its own channel and entity identity, including parallel failures. */
export function queryDiagnostics(result:QueryResult):UiText[] {
    return [
        ...result.diagnostics.map(coreDescriptionUiText),
        ...(result.engineDiagnostics??[]).flatMap(group=>group.diagnostics.map(value=>joinUiText([group.entityKey,engineDiagnosticUiText(value)],"\n"))),
        ...(result.productErrors??[]).map(group=>joinUiText([group.entityKey,apiErrorUiText(group.error)],"\n")),
    ];
}
