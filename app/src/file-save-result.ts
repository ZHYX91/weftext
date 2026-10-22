import { message as uiMessage } from "./ui-locale";
import { decodeApiErrorBody, type ApiErrorBody } from "./api-error";
import { decodeCoreError, type CoreError } from "./ui-diagnostics";
import { UiError } from "./ui-locale";
import type { OpenedDocument, Receipt } from "./types";

export interface FileSaveReceipt extends Receipt {
  committedSource?: string;
  document?: OpenedDocument;
  documentError?: ApiErrorBody;
  documentSourceMatchesCommit?: boolean;
}
export interface FrozenFileSave { operationId: string; source: string; sourcePath: string; path?: string }
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const text = (value: unknown): value is string => typeof value === "string" && value.length > 0;
const coordinate = (path: string): string => {
  const parts: string[] = [];
  for (const part of path.replaceAll("\\", "/").split("/")) {
    if (part === ".") continue;
    if (part === ".." && parts.length && parts.at(-1) !== "..") parts.pop(); else parts.push(part);
  }
  return parts.join("/").toLowerCase();
};
function document(value: unknown): value is OpenedDocument {
  if (!object(value) || !["sessionId", "baseId", "path", "root"].every(key => text(value[key])) || typeof value.source !== "string"
      || typeof value.kind !== "string" || !["managed", "managedInvalid", "managedTrash", "ordinary"].includes(value.kind)) return false;
  for (const key of ["nodeId", "workspaceId"]) if (value[key] !== undefined && !text(value[key])) return false;
  if (value.nodeId !== undefined && value.workspaceId === undefined) return false;
  for (const key of ["readOnly", "recoveredDraftConflict"]) if (value[key] !== undefined && typeof value[key] !== "boolean") return false;
  for (const key of ["recoveredDraft", "recoveredDraftBaseSource"]) if (value[key] !== undefined && value[key] !== null && typeof value[key] !== "string") return false;
  return value.recoveredDraftVariants === undefined || Array.isArray(value.recoveredDraftVariants) && value.recoveredDraftVariants.every(item =>
    object(item) && text(item.path) && typeof item.baseSource === "string" && typeof item.source === "string" && typeof item.baseKnown === "boolean");
}
/** One file-save delivery contract for the initial response and original-id
 * lookup. The generic operation envelope alone cannot acknowledge a writer. */
export function fileSaveReceipt(value: unknown, frozen: FrozenFileSave): FileSaveReceipt {
  const invalid = (): never => { throw new UiError(uiMessage("fileSave.invalidOriginalReceipt")); };
  const allowed = new Set(["operationId", "status", "path", "baseId", "revisionId", "recoveryIssue", "committedSource", "document", "documentError", "documentSourceMatchesCommit"]);
  if (!object(value) || Object.keys(value).some(key => !allowed.has(key)) || value.operationId !== frozen.operationId || typeof value.status !== "string" || !["saved", "committed"].includes(value.status)
      || !text(value.baseId) || !text(value.path)
      || Object.hasOwn(value,"revisionId") && !text(value.revisionId)
      || coordinate(value.path) !== coordinate(frozen.path ?? frozen.sourcePath)) return invalid();
  let recoveryIssue: CoreError | undefined;
  if (Object.hasOwn(value,"recoveryIssue")) {
    recoveryIssue = decodeCoreError(value.recoveryIssue);
    if (!recoveryIssue) return invalid();
  }
  let documentError: ApiErrorBody | undefined;
  if (Object.hasOwn(value,"documentError")) {
    documentError = decodeApiErrorBody(value.documentError);
    if (!documentError || value.document !== undefined || Object.hasOwn(value,"documentSourceMatchesCommit") && typeof value.documentSourceMatchesCommit !== "boolean") return invalid();
    if (value.committedSource !== undefined && value.committedSource !== frozen.source) return invalid();
  } else {
    if (!document(value.document) || value.committedSource !== frozen.source || typeof value.documentSourceMatchesCommit !== "boolean"
        || value.documentSourceMatchesCommit !== (value.document.source === frozen.source)
        || frozen.path !== undefined && coordinate(value.document.path) !== coordinate(value.path)) return invalid();
  }
  return { ...value, ...(recoveryIssue ? {recoveryIssue} : {}), ...(documentError ? {documentError} : {}) } as unknown as FileSaveReceipt;
}
