import { decodeProjectionResponse, type NoteProjection } from "./data-projection";
import type { DocumentBasis, PanelHost, SchemaRead } from "./native-value-form";
import { isUnicodeScalarText, message, UiError } from "./ui-locale";

async function sourceDigest(source: string): Promise<string> {
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(byte => byte.toString(16).padStart(2, "0")).join("");
}

/** A projection is a read of one submitted input. It cannot qualify a later
 * document/input, even when that input happens to contain the same bytes. */
export function sameProjectionInput(current: DocumentBasis | null, original: DocumentBasis): boolean {
    return !!current && current.inputId === original.inputId && current.sessionId === original.sessionId
        && current.source === original.source && current.serial === original.serial && current.baseId === original.baseId
        && current.path === original.path && current.root === original.root;
}

/** Pair the real local response with its submitted source and, when the caller
 * renders a separately read schema, that exact schema source/revision too. */
export async function readNoteProjection(request: PanelHost["request"], basis: Pick<DocumentBasis, "sessionId" | "source" | "serial">, schema?: Pick<SchemaRead, "source" | "revision">): Promise<NoteProjection> {
    const { sessionId, source, serial } = basis;
    const schemaSource = schema?.source, schemaRevision = schema?.revision;
    const invalid = (): never => { throw new UiError(message("error.invalidResponse")); };
    if (typeof source !== "string" || !isUnicodeScalarText(source) || !Number.isSafeInteger(serial) || serial < 0
        || schema !== undefined && (typeof schemaSource !== "string" || !isUnicodeScalarText(schemaSource) || schemaRevision !== null && typeof schemaRevision !== "string")) return invalid();
    const [wire, sourceHash, schemaHash] = await Promise.all([
        request<unknown>("/api/data/project", { sessionId, source, serial }),
        sourceDigest(source),
        schema === undefined ? Promise.resolve(undefined) : sourceDigest(schemaSource!),
    ]);
    const response = decodeProjectionResponse(wire);
    if (response.serial !== serial || response.sourceHash !== sourceHash
        || schemaHash !== undefined && (response.schemaHash !== schemaHash || response.schemaRevision !== schemaRevision)) return invalid();
    return { ...response.projection, readOnly: response.readOnly };
}
