export type ViewMode = "source" | "styled" | "focus" | "write" | "read";

/** Read-only Core provenance. Missing metadata grants no identity to a local comparison copy. */
export interface DraftInput {
  path: string;
  baseSource: string;
  source: string;
  baseKnown: boolean;
  producerId?: string;
  inputId?: string;
  role?: { kind: "proposal" } | { kind: "committedBaseline"; operationId: string };
}

export interface OpenedDocument {
    viewInputId?: string;
  sessionId: string;
  path: string;
  root: string;
  source: string;
  baseId: string;
  kind: string;
  readOnly?: boolean;
  nodeId?: string;
  workspaceId?: string;
  recoveredDraft?: string | null;
  recoveredDraftBaseSource?: string | null;
  recoveredDraftWitness?: string | null;
  recoveredDraftConflict?: boolean;
  recoveredDraftVariants?: DraftInput[];
}
export interface InventoryEntry {
  path: string;
  relativePath: string;
  name: string;
  parentPath: string;
  kind: "directory" | "node" | "file" | "blocked";
  identityStatus: string;
  nodeId?: string;
  title?: string;
  excerpt?: string;
  source?: string;
}
export interface Inventory {
  root: string;
  workspaceId?: string;
  status: string;
  entries: InventoryEntry[];
  warnings: CoreDescription[];
  complete: boolean;
}
export interface Origin {
  path: string;
  from: number;
  to: number;
  utf16From: number;
  utf16To: number;
  raw: string;
  syntaxKind: string;
  isMain: boolean;
}
export interface RenderNode {
  key: string;
  kind: "element" | "text" | "frame" | "tag";
  tag: string | null;
  text: string | null;
  children: RenderNode[];
  origin: Origin | null;
  writable: boolean;
  attrs: Record<string, unknown>;
}
export interface SyntaxToken {
  kind: string;
  from: number;
  to: number;
  utf16From: number;
  utf16To: number;
}
export interface SourceInfo {
  notes?: import("./document-notes").NotesSourceMap;
  syntaxValid: boolean;
  identityStatus: string;
  id?: string;
  types: string[];
  title?: string;
  headings: unknown[];
  links: unknown[];
  syntaxTokens: SyntaxToken[];
  bodyInsertion?: { from: number; to: number; utf16From: number; utf16To: number; raw: string; writable: boolean } | null;
}
export type Diagnostic = EngineDiagnostic;
export interface RenderOutput {
  headingStructure: import("./heading-structure-state").HeadingSnapshot;
  ok: boolean;
  html: string;
  tree: RenderNode | null;
  diagnostics: Diagnostic[];
  sourceInfo: SourceInfo;
  dependencies: { path: string; kind: string }[];
  fontFamilies: string[];
  compilerVersion: string;
  serial: number;
  source: string;
}
export interface Receipt { recoveryIssue?: CoreError; operationId: string; status: string; path: string; baseId: string; revisionId?: string }
export interface RecoveryImage { present: boolean; source?: string; base64?: string; byteLength: number; sha256?: string }
export interface RecoveryInfo {
  operationId: string; path: string; status: string; beforeSource?: string; afterSource?: string; currentSource?: string; description: CoreDescription;
  coherence?: string; allowedActions?: string[];
  members?: { kind: string; path: string; targetPath?: string; before: RecoveryImage; after: RecoveryImage; current: RecoveryImage; identityBinding: string; state: string; description?: CoreDescription }[];
}
import type { CoreDescription, CoreError, EngineDiagnostic, ErrorDescription } from "./ui-diagnostics";
