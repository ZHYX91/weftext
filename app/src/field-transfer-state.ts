import type { FieldProjection, NoteProjection } from "./data-projection";
import { resolveShape, type DocumentBasis, type SchemaRead, type SourcePatch } from "./native-value-form";
import { exactKeys } from "./ui-diagnostics";
import { isUnicodeScalarText, message, UiError } from "./ui-locale";

/** A projection of one local whole-document author input, never a narrow field grant. */
export interface FieldTransferContext {
    basis: DocumentBasis;
    schemaRead: SchemaRead & { semanticDigest: string };
    projection: NoteProjection;
}
export interface FieldTransferSelection { fromField: string; toField: string; start: number; end: number; to: number; }
export interface FieldTransferInputs { fromField: string; toField: string; first: string; last: string; position: string; }
export type TransferList = FieldProjection & { cell: { state: "value"; value: { kind: "array"; value: import("./native-values").NativeValue[] } } };

export function transferLists(context: FieldTransferContext): TransferList[] {
    const projection = context.projection;
    if (projection.readOnly || projection.qualification !== "qualified" || projection.budgetExhausted || projection.diagnostics.length) return [];
    return projection.fields.filter((field): field is TransferList => {
        const schemaField = context.schemaRead.schema.fields.find(item => item.id === field.fieldId);
        return !!schemaField && resolveShape(schemaField.shape, context.schemaRead.schema).kind === "list"
            && field.writable && !!field.sourceRange && typeof field.raw === "string" && !field.unbound?.length
            && field.cell.state === "value" && field.cell.value.kind === "array";
    });
}

export function transferContextKey(context: FieldTransferContext): string {
    const basis = context.basis;
    return JSON.stringify([basis.inputId, basis.sessionId, basis.source, basis.serial, basis.baseId, basis.path, basis.root,
        basis.workspaceId, basis.nodeId, basis.inputRevision, context.schemaRead.source, context.schemaRead.revision, context.schemaRead.semanticDigest,
        context.projection.id, context.projection.qualification, context.projection.readOnly, context.projection.budgetExhausted,
        context.projection.fields.map(field => [field.fieldId, field.writable, field.unbound, field.cell.state])]);
}

/** Inputs remain complete visible ordinals; no Number/trim coercion can retarget a row. */
function ordinal(text: string, limit: number): number | null {
    if (!/^[1-9][0-9]*$/.test(text)) return null;
    const value = Number(text);
    return Number.isSafeInteger(value) && value <= limit ? value - 1 : null;
}
export function fieldTransferSelection(context: FieldTransferContext, input: FieldTransferInputs): FieldTransferSelection {
    if (input.fromField === input.toField) throw new UiError(message("error.data.fieldTransfer.differentFieldsRequired"));
    const lists = transferLists(context), from = lists.find(field => field.fieldId === input.fromField), to = lists.find(field => field.fieldId === input.toField);
    if (!from || !to) throw new UiError(message("error.data.fieldTransfer.existingListsRequired"));
    const start = ordinal(input.first, from.cell.value.value.length), end = ordinal(input.last, from.cell.value.value.length), position = ordinal(input.position, to.cell.value.value.length + 1);
    if (start === null || end === null || position === null || end < start) throw new UiError(message("error.data.fieldTransfer.ordinalInvalid"));
    return { fromField: from.fieldId, toField: to.fieldId, start, end, to: position };
}

export async function fieldTransferSourceHash(source: string): Promise<string> {
    if (!isUnicodeScalarText(source)) throw new UiError(message("error.invalidResponse"));
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(byte => byte.toString(16).padStart(2, "0")).join("");
}
const object = (value: unknown): Record<string, unknown> | undefined => value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : undefined;

/** The sole patch is bound to this exact original input, schema and ordinal request. */
export function decodeFieldTransferPatch(value: unknown, context: FieldTransferContext, selection: FieldTransferSelection, sourceHash: string): SourcePatch {
    const fail = (): never => { throw new UiError(message("error.invalidResponse")); };
    const result = object(value), wireSelection = result && object(result.transfer);
    if (!result || !exactKeys(result, ["source", "patches", "patchUnit", "serial", "sourceHash", "schemaRevision", "semanticDigest", "transfer"])
        || !wireSelection || !exactKeys(wireSelection, ["fromField", "toField", "start", "end", "to"])
        || Object.entries(selection).some(([key, selected]) => wireSelection[key] !== selected)
        || result.serial !== context.basis.serial || result.sourceHash !== sourceHash || result.patchUnit !== "utf16"
        || result.schemaRevision !== context.schemaRead.revision || result.semanticDigest !== context.schemaRead.semanticDigest
        || typeof result.source !== "string" || !isUnicodeScalarText(result.source) || !Array.isArray(result.patches) || result.patches.length !== 1) return fail();
    const patch = object(result.patches[0]), source = context.basis.source;
    if (!patch || !exactKeys(patch, ["from", "to", "text", "byteFrom", "byteTo", "expected"])
        || !Number.isSafeInteger(patch.from) || !Number.isSafeInteger(patch.to)
        || (patch.from as number) < 0 || (patch.to as number) < (patch.from as number) || (patch.to as number) > source.length
        || typeof patch.text !== "string" || !isUnicodeScalarText(patch.text)) return fail();
    const from = patch.from as number, to = patch.to as number;
    if (!isUnicodeScalarText(source.slice(0, from)) || !isUnicodeScalarText(source.slice(to))
        || patch.expected !== source.slice(from, to)
        || patch.byteFrom !== new TextEncoder().encode(source.slice(0, from)).length || patch.byteTo !== new TextEncoder().encode(source.slice(0, to)).length
        || source.slice(0, from) + patch.text + source.slice(to) !== result.source) return fail();
    return { from, to, text: patch.text };
}
