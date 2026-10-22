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
const object = (value:unknown): Record<string,unknown>|undefined => value && typeof value === "object" && !Array.isArray(value) ? value as Record<string,unknown> : undefined;
const strings = (value:unknown): value is string[] => Array.isArray(value) && value.every(item=>typeof item === "string");
const range = (value:unknown): value is {from:number;to:number} => { const item=object(value); return !!item && exactKeys(item,["from","to"]) && Number.isSafeInteger(item.from) && Number.isSafeInteger(item.to) && (item.from as number)>=0 && (item.to as number)>=(item.from as number); };

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

export function decodeProjection(value:unknown): NoteProjection {
    const fail = ():never => {throw new UiError(message("error.invalidResponse"));};
    let item=object(value); if(!item) return fail();
    let readOnly: boolean|undefined;
    if(Object.hasOwn(item,"projection")) {
        if(!exactKeys(item,["projection"],["readOnly"]) || Object.hasOwn(item,"readOnly") && typeof item.readOnly !== "boolean") return fail();
        readOnly=item.readOnly as boolean|undefined; item=object(item.projection); if(!item) return fail();
    }
    const diagnostics=decodeErrorDescriptions(item.diagnostics);
    if(!exactKeys(item,["qualification","id","types","resolvedTypes","fields","unknownFields","diagnostics","budgetExhausted"],["readOnly"]) || typeof item.qualification !== "string" || item.id !== null && typeof item.id !== "string" || !strings(item.types) || !strings(item.resolvedTypes) || !Array.isArray(item.fields) || !Array.isArray(item.unknownFields) || !diagnostics || typeof item.budgetExhausted !== "boolean" || Object.hasOwn(item,"readOnly") && typeof item.readOnly !== "boolean") return fail();
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
    return {qualification:item.qualification,id:item.id as string|null,types:[...item.types],resolvedTypes:[...item.resolvedTypes],fields,unknownFields,diagnostics,budgetExhausted:item.budgetExhausted, ...(readOnly !== undefined || typeof item.readOnly === "boolean" ? {readOnly:readOnly === true || item.readOnly === true} : {})};
}
