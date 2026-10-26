import { decodeNativeValue, type NativeValue } from "./native-values";
import { decodeErrorDescription, decodeErrorDescriptions, exactKeys, type DataErrorKind, type ErrorDescription } from "./ui-diagnostics";
import { UiError, message } from "./ui-locale";

export type Cell = (
    | {state:"value";value:NativeValue}
    | {state:"absent"|"null"}
    | {state:"invalid";value:NativeValue|null;diagnostics:readonly ErrorDescription[]}
    | {state:"unavailable";kind:DataErrorKind;reason:ErrorDescription;diagnostics:readonly ErrorDescription[]}
) & {value?:NativeValue|null;diagnostics?:readonly ErrorDescription[];reason?:ErrorDescription};
export interface FieldProjection {
    fieldId:string;key:string;cell:Cell;sourceRange?:{from:number;to:number}|null;writable:boolean;raw?:string|null;unbound?:string[];required?:boolean;
}
export interface NoteProjection {
    qualification:string;id?:string|null;types:string[];resolvedTypes:string[];fields:FieldProjection[];
    unknownFields:{key:string;raw:string;range?:{from:number;to:number};value?:NativeValue|null}[];
    diagnostics:readonly ErrorDescription[];budgetExhausted:boolean;readOnly?:boolean;
}
export interface ProjectionResponse {
    readonly projection: NoteProjection;
    readonly serial: number;
    readonly sourceHash: string;
    readonly schemaRevision: string|null;
    readonly schemaHash: string;
    readonly readOnly: boolean;
}
const object = (value:unknown): Record<string,unknown>|undefined => value && typeof value === "object" && !Array.isArray(value) ? value as Record<string,unknown> : undefined;
const strings = (value:unknown): value is string[] => Array.isArray(value) && value.every(item=>typeof item === "string");
const range = (value:unknown): value is {from:number;to:number} => { const item=object(value); return !!item && exactKeys(item,["from","to"]) && Number.isSafeInteger(item.from) && Number.isSafeInteger(item.to) && (item.from as number)>=0 && (item.to as number)>=(item.from as number); };
const hash = (value:unknown): value is string => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
function invalidProjection(): never { throw new UiError(message("error.invalidResponse")); }

export function decodeCell(value:unknown): Cell|undefined {
    const item=object(value); if(!item) return;
    if(item.state === "absent" || item.state === "null") return exactKeys(item,["state"]) ? {state:item.state} : undefined;
    if(item.state === "value") {const native=decodeNativeValue(item.value); return exactKeys(item,["state","value"]) && native ? {state:"value",value:native} : undefined;}
    const diagnostics=decodeErrorDescriptions(item.diagnostics); if(!diagnostics) return;
    if(item.state === "invalid") {
        const native=item.value === null ? null : decodeNativeValue(item.value);
        return exactKeys(item,["state","diagnostics","value"]) && native !== undefined ? {state:"invalid",diagnostics,value:native} : undefined;
    }
    if(item.state === "unavailable") {
        const reason=decodeErrorDescription(item.reason);
        if(exactKeys(item,["state","kind","reason","diagnostics"]) && reason && ["invalid","unavailable","coverage","budget","nativeBudget","closedExpressionUnavailable"].includes(String(item.kind))) return {state:"unavailable",kind:item.kind as DataErrorKind,reason,diagnostics};
    }
}

/** Decode the Data domain value. Transport metadata belongs to its route's response. */
export function decodeProjection(value:unknown): NoteProjection {
    const fail = invalidProjection;
    const item=object(value); if(!item) return fail();
    const diagnostics=decodeErrorDescriptions(item.diagnostics);
    if(!exactKeys(item,["qualification","id","types","resolvedTypes","fields","unknownFields","diagnostics","budgetExhausted"]) || typeof item.qualification !== "string" || item.id !== null && typeof item.id !== "string" || !strings(item.types) || !strings(item.resolvedTypes) || !Array.isArray(item.fields) || !Array.isArray(item.unknownFields) || !diagnostics || typeof item.budgetExhausted !== "boolean") return fail();
    const fields:FieldProjection[]=[];
    for(const value of item.fields) {
        const field=object(value),cell=field && decodeCell(field.cell);
        if(!field || !cell || !exactKeys(field,["fieldId","key","cell","sourceRange","raw","writable","required","unbound"]) || typeof field.fieldId !== "string" || typeof field.key !== "string" || field.sourceRange !== null && !range(field.sourceRange) || field.raw !== null && typeof field.raw !== "string" || typeof field.writable !== "boolean" || typeof field.required !== "boolean" || !strings(field.unbound)) return fail();
        fields.push({...field,cell} as unknown as FieldProjection);
    }
    const unknownFields:NoteProjection["unknownFields"]=[];
    for(const value of item.unknownFields) {
        const field=object(value),native=field && (field.value===null ? null : decodeNativeValue(field.value));
        if(!field || !exactKeys(field,["key","raw","range","value"]) || typeof field.key !== "string" || typeof field.raw !== "string" || !range(field.range) || native===undefined) return fail();
        unknownFields.push({key:field.key,raw:field.raw,range:field.range,value:native});
    }
    return {qualification:item.qualification,id:item.id as string|null,types:[...item.types],resolvedTypes:[...item.resolvedTypes],fields,unknownFields,diagnostics,budgetExhausted:item.budgetExhausted};
}

/** The local /api/data/project response retains the exact snapshot and schema
 * metadata for the caller to bind before consuming the projection. */
export function decodeProjectionResponse(value:unknown): ProjectionResponse {
    const item=object(value);
    if(!item || !exactKeys(item,["projection","serial","sourceHash","schemaRevision","schemaHash","readOnly"])
        || typeof item.serial !== "number" || !Number.isSafeInteger(item.serial) || item.serial < 0
        || !hash(item.sourceHash) || !hash(item.schemaHash)
        || item.schemaRevision !== null && typeof item.schemaRevision !== "string"
        || typeof item.readOnly !== "boolean") return invalidProjection();
    return {projection:decodeProjection(item.projection),serial:item.serial,sourceHash:item.sourceHash,
        schemaRevision:item.schemaRevision as string|null,schemaHash:item.schemaHash,readOnly:item.readOnly};
}
