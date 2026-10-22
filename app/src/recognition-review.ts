import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Recognition evidence is immutable. This draft is a proposal for the sole Core author. */
export type RecognitionRect = [
    number,
    number,
    number,
    number
];
export interface RecognitionBlock {
    id: string;
    page?: number;
    blockIndex?: number;
    text: string;
    bbox?: RecognitionRect;
    sourceMethod: "text-layer" | "ocr" | "docx";
    confidence?: number | null;
    words?: {
        text: string;
        bbox: RecognitionRect;
        confidence: number | null;
    }[];
}
export interface RecognitionTable {
    kind: string;
    rows: number;
    columns: number;
    bbox: RecognitionRect;
    cells: {
        row: number;
        column: number;
        text: string;
        bbox: RecognitionRect;
        wordIndices: number[];
    }[];
}
export interface RecognitionRegion {
    id: string;
    rect: RecognitionRect;
    method: string;
    status: string;
    rawText: string;
    blocks: RecognitionBlock[];
    tables?: RecognitionTable[];
    losses: string[];
}
export interface RecognitionPage {
    page: number;
    status: string;
    width?: number;
    height?: number;
    units?: string;
    rotation?: number;
    coordinateSpace?: string;
    regions: RecognitionRegion[];
    losses: string[];
    readingOrder?: {
        original: string[];
        suggested: string[];
        headerFooterCandidates: string[];
        kind: string;
    };
}
export interface RecognitionEvidence {
    version: number;
    status: "complete" | "partial" | "failed";
    input: {
        sha256: string;
        bytes: number;
        mediaType: string;
    };
    pageCount?: number | null;
    pages: RecognitionPage[];
    blocks?: RecognitionBlock[];
    docxBlocks?: RecognitionBlock[];
    losses?: string[];
    issues?: {
        code: string;
        blockId?: string;
        elements?: unknown;
    }[];
    error?: string;
    runtime?: unknown;
    selection?: unknown;
    workerFinalValidation?: boolean;
}
export interface RecognitionFieldCandidate {
    raw: string;
    parsed?: string | null;
    parseStatus: string;
    sources: {
        page?: number;
        bbox?: RecognitionRect;
        quote?: string;
        blockId?: string;
    }[];
    reason?: string;
}
export interface RecognitionScheme {
    scheme: {
        id: string;
        name: string;
        version: number;
    };
    applicability: string;
    reason?: string;
    fields: Record<string, {
        status: string;
        value?: string;
        candidates: RecognitionFieldCandidate[];
    }>;
    issues: {
        code: string;
        severity: string;
    }[];
    structures?: {
        id: string;
        kind: string;
        raw: string;
        content: string;
        level?: number | null;
        sources: {
            blockId?: string;
            page?: number;
            bbox?: RecognitionRect;
        }[];
        proof: string;
    }[];
}
export interface RecognitionReview {
    blocks: {
        id: string;
        text: string;
        included: boolean;
    }[];
    tables: {
        page: number;
        regionId: string;
        index: number;
        accepted: boolean;
        cells: string[][];
        replacedBlocks?: string[];
    }[];
    fields: {
        key: string;
        candidate: number;
        value: string;
        confirmed: boolean;
        manualCorrection?: boolean;
    }[];
    structures: string[];
    numbering: "keep" | "remove" | "rebuild";
    acceptPartial: boolean;
    acceptLosses: boolean;
}
export const recognitionBlocks = (evidence: RecognitionEvidence): RecognitionBlock[] => evidence.docxBlocks ?? evidence.blocks ?? evidence.pages.flatMap(page => page.regions.flatMap(region => region.blocks));
const copy = <T>(value: T): T => structuredClone(value);
export function initialRecognitionReview(evidence: RecognitionEvidence, scheme?: RecognitionScheme): RecognitionReview {
    const blocks = recognitionBlocks(evidence);
    const ids = new Set(blocks.map(b => b.id));
    if (ids.size !== blocks.length || blocks.some(b => !b.id || typeof b.text !== "string"))
        throw new UiError(uiMessage("recognitionReview.blockIdentityOrTextInvalid"));
    return { blocks: blocks.map(block => ({ id: block.id, text: block.text, included: true })),
        tables: evidence.pages.flatMap(page => page.regions.flatMap(region => (region.tables ?? []).map((table, index) => ({ page: page.page, regionId: region.id, index, accepted: false, cells: Array.from({ length: table.rows }, (_, row) => Array.from({ length: table.columns }, (_, column) => table.cells.find(c => c.row === row && c.column === column)?.text ?? "")) })))),
        fields: Object.keys(scheme?.fields ?? {}).map(key => ({ key, candidate: -1, value: "", confirmed: false, manualCorrection: false })), structures: [], numbering: "keep", acceptPartial: false, acceptLosses: false };
}
/** Small bounded undo stack; it never changes the read-only worker result. */
export class RecognitionReviewDraft {
    private past: RecognitionReview[] = [];
    private future: RecognitionReview[] = [];
    private current: RecognitionReview;
    revision = 0;
    constructor(readonly evidence: RecognitionEvidence, readonly scheme?: RecognitionScheme) { this.current = initialRecognitionReview(evidence, scheme); }
    get value(): RecognitionReview { return copy(this.current); }
    get canUndo(): boolean { return this.past.length > 0; }
    get canRedo(): boolean { return this.future.length > 0; }
    change(edit: (draft: RecognitionReview) => void): void {
        const next = this.value;
        edit(next);
        if (JSON.stringify(next) === JSON.stringify(this.current))
            return;
        this.past.push(this.current);
        if (this.past.length > 80)
            this.past.shift();
        this.current = next;
        this.future = [];
        this.revision++;
    }
    undo(): void {
        const value = this.past.pop();
        if (!value)
            return;
        this.future.push(this.current);
        this.current = value;
        this.revision++;
    }
    redo(): void {
        const value = this.future.pop();
        if (!value)
            return;
        this.past.push(this.current);
        this.current = value;
        this.revision++;
    }
    move(id: string, delta: -1 | 1): void {
        this.change(draft => {
            const from = draft.blocks.findIndex(b => b.id === id), to = from + delta;
            if (from < 0 || to < 0 || to >= draft.blocks.length)
                return;
            [draft.blocks[from], draft.blocks[to]] = [draft.blocks[to], draft.blocks[from]];
        });
    }
    adoptOrder(page: RecognitionPage): void {
        this.change(draft => {
            const order = page.readingOrder?.suggested;
            if (!order)
                return;
            const selected = draft.blocks.filter(b => order.includes(b.id));
            if (new Set(order).size !== order.length || selected.length !== order.length)
                throw new UiError(uiMessage("recognitionReview.suggestedOrderMismatch"));
            const sorted = order.map(id => selected.find(b => b.id === id)!);
            let index = 0;
            draft.blocks = draft.blocks.map(b => order.includes(b.id) ? sorted[index++] : b);
        });
    }
    selectField(key: string, candidate: number): void {
        const field = this.scheme?.fields[key], item = field?.candidates[candidate], current = this.current.fields.find(f => f.key === key);
        if (!item || this.scheme?.applicability !== "supported" || !current?.manualCorrection && (item.parseStatus !== "parsed" || typeof item.parsed !== "string"))
            throw new UiError(uiMessage("recognitionReview.candidateExactValueMissing"));
        this.change(draft => {
            const next = draft.fields.find(f => f.key === key);
            if (!next)
                throw new UiError(uiMessage("recognitionReview.fieldUnknown"));
            Object.assign(next, { candidate, value: typeof item.parsed === "string" ? item.parsed : item.raw, confirmed: false });
        });
    }
    confirmField(key: string, confirmed: boolean): void {
        this.change(draft => {
            const next = draft.fields.find(f => f.key === key), candidate = this.scheme?.fields[key]?.candidates[next?.candidate ?? -1];
            if (!next || this.scheme?.applicability !== "supported" || !next.value.trim() || !next.manualCorrection && (next.candidate < 0 || candidate?.parseStatus !== "parsed" || candidate.parsed !== next.value))
                throw new UiError(uiMessage("recognitionReview.selectCandidateBeforeEditing"));
            next.confirmed = confirmed;
        });
    }
    acceptTable(index: number, accepted: boolean): void {
        this.change(draft => {
            const table = draft.tables[index];
            if (!table)
                throw new UiError(uiMessage("recognitionReview.tableCandidateMissing"));
            if (!accepted) {
                for (const id of table.replacedBlocks ?? []) {
                    const block = draft.blocks.find(b => b.id === id);
                    if (block)
                        block.included = true;
                }
                table.accepted = false;
                table.replacedBlocks = [];
                return;
            }
            const region = this.evidence.pages.find(p => p.page === table.page)?.regions.find(r => r.id === table.regionId), original = region?.tables?.[table.index];
            if (!region || !original)
                throw new UiError(uiMessage("recognitionReview.tableSourceMissing"));
            const box = original.bbox, intersects = (a: RecognitionRect, b: RecognitionRect): boolean => a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1];
            const overlapping = region.blocks.filter(b => b.bbox && intersects(b.bbox, box));
            if (overlapping.some(b => b.bbox![0] < box[0] || b.bbox![1] < box[1] || b.bbox![2] > box[2] || b.bbox![3] > box[3]))
                throw new UiError(uiMessage("recognitionReview.tableTextBlocksShared"));
            if (!overlapping.length)
                throw new UiError(uiMessage("recognitionReview.tableBlocksMismatch"));
            table.replacedBlocks = overlapping.filter(b => draft.blocks.find(d => d.id === b.id)?.included).map(b => b.id);
            for (const b of draft.blocks)
                if (table.replacedBlocks.includes(b.id))
                    b.included = false;
            table.accepted = true;
        });
    }
}
export function recognitionReviewReady(evidence: RecognitionEvidence, review: RecognitionReview): boolean {
    return evidence.status !== "failed" && (evidence.status === "complete" || review.acceptPartial) && review.acceptLosses;
}
export function confidenceLabel(value?: number | null): UiText { return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 100 ? displayMessage("recognitionReview.backendScore", { p0: uiDisplayParameter(value.toFixed(1)) }) : uiMessage("recognitionReview.confidenceUnknown"); }
export function parseRecognitionPages(value: string): number[] {
    if (!value.trim())
        throw new UiError(uiMessage("recognitionReview.pageInputRequired"));
    const pages: number[] = [];
    for (const token of value.split(/[,，]/)) {
        const match = /^\s*(\d+)\s*(?:-\s*(\d+))?\s*$/.exec(token);
        if (!match)
            throw new UiError(uiMessage("recognitionReview.pageFormatInvalid"));
        const start = Number(match[1]), end = Number(match[2] ?? match[1]);
        if (start < 1 || end > 100 || end < start)
            throw new UiError(uiMessage("recognitionReview.pageRangeInvalid"));
        for (let page = start; page <= end; page++) {
            if (pages.includes(page))
                throw new UiError(uiMessage("recognitionReview.duplicatePage"));
            pages.push(page);
        }
    }
    return pages;
}
export function parseRecognitionRect(value: string): RecognitionRect | undefined {
    if (!value.trim())
        return;
    const values = value.split(/[,，]/).map(Number);
    if (values.length !== 4 || values.some(n => !Number.isFinite(n) || n < 0 || n >= 100000) || values[2] <= values[0] || values[3] <= values[1])
        throw new UiError(uiMessage("recognitionReview.regionInvalid"));
    return values as RecognitionRect;
}
