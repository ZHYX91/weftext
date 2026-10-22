import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError } from "./ui-locale";
import { setUiText, type UiText } from "./ui-locale";
import type { Diagnostic } from "./types";
export interface PagedBasis {
    sessionId: string;
    source: string;
    serial: number;
    /** Optional fixed compilation-environment key supplied by the host. */
    environment?: string;
}
export interface PagedOutput {
    ok: boolean;
    target: "paged";
    pdfBase64: string | null;
    pageCount: number;
    diagnostics: Diagnostic[];
    dependencies: {
        path: string;
        kind: string;
        status?: string;
    }[];
    fontFamilies: string[];
    compilerVersion: string;
    source: string;
    serial: number;
    environment?: string;
}
export interface PagedState {
    pending: boolean;
    current: boolean;
    pageCount: number;
    diagnostics: Diagnostic[];
    error: string | null;
}
export interface PagedPreviewOptions {
    basis(): PagedBasis | null;
    request(basis: PagedBasis): Promise<PagedOutput>;
    fileName?(): string;
    onState?(state: PagedState): void;
    onError?(error: Error): void;
}
const PDF_BYTES_LIMIT = 16 * 1024 * 1024;
const PDF_BASE64_LIMIT = Math.ceil(PDF_BYTES_LIMIT / 3) * 4;
/** Exact source equality matters even if a host accidentally reuses a serial. */
export function samePagedBasis(left: PagedBasis, right: PagedBasis | null): boolean {
    return !!right && left.sessionId === right.sessionId && left.serial === right.serial
        && left.source === right.source && left.environment === right.environment;
}
export function pagedResultMatches(basis: PagedBasis, result: PagedOutput): boolean {
    return result.source === basis.source && result.serial === basis.serial
        && result.target === "paged" && result.compilerVersion === "0.15.1"
        && (basis.environment === undefined || result.environment === basis.environment);
}
/** Bounded PDF bytes only; neither a server URL nor author HTML enters a frame. */
export function decodePagedPdf(base64: string): Uint8Array<ArrayBuffer> {
    if (!base64 || base64.length > PDF_BASE64_LIMIT || base64.length % 4 !== 0
        || !/^[A-Za-z0-9+/]+={0,2}$/.test(base64))
        throw new UiError(uiMessage("pagedPreview.pdfDataInvalid"));
    let decoded: string;
    try {
        decoded = atob(base64);
    }
    catch {
        throw new UiError(uiMessage("pagedPreview.pdfDecodeFailed"));
    }
    if (decoded.length > PDF_BYTES_LIMIT || !decoded.startsWith("%PDF-"))
        throw new UiError(uiMessage("pagedPreview.pdfUnsupportedOrOversized"));
    const bytes = new Uint8Array(decoded.length);
    for (let index = 0; index < decoded.length; index++)
        bytes[index] = decoded.charCodeAt(index);
    return bytes;
}
export function pagedFileName(path: string): string {
    const name = path.replaceAll("\\", "/").split("/").at(-1) || "草稿.typ";
    return name.replace(/\.[^.]*$/, "").replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_") + ".pdf";
}
/**
 * Read-only native Paged preview. The browser owns PDF selection, page
 * navigation, zoom and printing; no PDF text or coordinates grant a writer.
 * Import paged-preview.css in the host's frontend entry point.
 */
export class PagedPreview {
    readonly element: HTMLElement;
    private readonly frame: HTMLIFrameElement;
    private readonly status: HTMLElement;
    private readonly freshness: HTMLElement;
    private readonly download: HTMLAnchorElement;
    private readonly open: HTMLAnchorElement;
    private readonly help: HTMLElement;
    private readonly diagnosticDetails: HTMLDetailsElement;
    private generation = 0;
    private pending = false;
    private disposed = false;
    private artifact: {
        basis: PagedBasis;
        output: PagedOutput;
        url: string;
    } | null = null;
    private diagnostics: Diagnostic[] = [];
    private error: string | null = null;
    constructor(host: HTMLElement, private readonly options: PagedPreviewOptions) {
        this.element = document.createElement("section");
        this.element.className = "paged-preview";
        setUiAttr(this.element, "aria-label", uiMessage("pagedPreview.draftPdfAria"));
        const toolbar = document.createElement("div");
        toolbar.className = "paged-toolbar";
        this.freshness = document.createElement("span");
        this.freshness.className = "paged-freshness";
        this.freshness.setAttribute("role", "status");
        const actions = document.createElement("div");
        actions.className = "paged-actions";
        this.download = document.createElement("a");
        this.download.className = "button-secondary compact";
        setUiText(this.download, uiMessage("pagedPreview.downloadPdf"));
        this.open = document.createElement("a");
        this.open.className = "button-secondary compact";
        setUiText(this.open, uiMessage("pagedPreview.openPdfPrint"));
        this.open.target = "_blank";
        this.open.rel = "noopener noreferrer";
        setUiAttr(this.open, "title", uiMessage("pagedPreview.openPdfPrintTitle"));
        this.download.addEventListener("click", event => this.guardAction(event));
        this.open.addEventListener("click", event => {
            if (this.guardAction(event)) {
                setUiText(this.help, uiMessage("pagedPreview.viewerPrintHint"));
            }
        });
        actions.append(this.download, this.open);
        toolbar.append(this.freshness, actions);
        this.status = document.createElement("div");
        this.status.className = "paged-status";
        this.status.setAttribute("role", "status");
        this.status.setAttribute("aria-live", "polite");
        this.frame = document.createElement("iframe");
        this.frame.className = "paged-viewer";
        setUiAttr(this.frame, "title", uiMessage("pagedPreview.nativeDraftPdfTitle"));
        this.frame.setAttribute("referrerpolicy", "no-referrer");
        this.help = document.createElement("p");
        this.help.className = "paged-help";
        this.diagnosticDetails = document.createElement("details");
        this.diagnosticDetails.className = "paged-diagnostics";
        this.element.append(toolbar, this.status, this.frame, this.help, this.diagnosticDetails);
        host.append(this.element);
        this.clearArtifact();
        this.showState();
    }
    get state(): PagedState {
        return {
            pending: this.pending,
            current: !!this.artifact && samePagedBasis(this.artifact.basis, this.options.basis()),
            pageCount: this.artifact?.output.pageCount ?? 0,
            diagnostics: this.diagnostics,
            error: this.error,
        };
    }
    async render(proposedBasis?: PagedBasis): Promise<void> {
        if (this.disposed)
            return;
        const capture = proposedBasis ?? this.options.basis();
        if (!capture || !samePagedBasis(capture, this.options.basis()))
            return;
        const basis = { ...capture };
        const generation = ++this.generation;
        this.clearArtifact();
        this.pending = true;
        this.error = null;
        this.diagnostics = [];
        setUiText(this.status, uiMessage("pagedPreview.compilingNativeDraft"));
        this.showState();
        try {
            const output = await this.options.request({ ...basis });
            if (this.disposed || generation !== this.generation)
                return;
            if (!samePagedBasis(basis, this.options.basis())) {
                setUiText(this.status, uiMessage("pagedPreview.draftChangedAwaitingResult"));
                return;
            }
            if (!pagedResultMatches(basis, output))
                throw new UiError(uiMessage("pagedPreview.resultStaleAwaitingCurrent"));
            this.diagnostics = output.diagnostics;
            if (!output.ok || !output.pdfBase64) {
                const diagnostic = output.diagnostics.find(item => item.severity === "error"), detail = diagnostic && engineDiagnosticUiText(diagnostic);
                throw new UiError(detail ? displayMessage("pagedPreview.previewUnavailableDetail", { p0: uiDisplayParameter(detail) }) : uiMessage("pagedPreview.draftPdfUnavailable"));
            }
            if (!Number.isSafeInteger(output.pageCount) || output.pageCount < 1 || output.pageCount > 1000)
                throw new UiError(uiMessage("pagedPreview.pageCountInvalid"));
            const bytes = decodePagedPdf(output.pdfBase64);
            const url = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
            this.artifact = { basis: { ...basis }, output, url };
            this.frame.src = `${url}#view=FitH`;
            this.frame.hidden = false;
            setUiAttr(this.frame, "title", displayMessage("pagedPreview.nativeDraftPdfRevisionTitle", { p0: basis.serial, p1: output.pageCount }));
            this.download.href = url;
            this.download.download = pagedFileName(this.options.fileName?.() ?? "草稿.typ");
            this.open.href = url;
            setUiText(this.help, uiMessage("pagedPreview.viewerControlsHint"));
            setUiText(this.status, "");
        }
        catch (error) {
            if (!this.disposed && generation === this.generation && samePagedBasis(basis, this.options.basis())) {
                this.error = error instanceof Error ? error.message : String(error);
                setUiText(this.status, this.error);
                this.options.onError?.(error instanceof Error ? error : new UiError(this.error));
            }
        }
        finally {
            if (!this.disposed && generation === this.generation) {
                this.pending = false;
                this.showState();
            }
        }
    }
    /** Also call on style, dependency-root, session, or environment changes. */
    invalidate(reason: UiText = uiMessage("pagedPreview.basisChangedRecompile")): void {
        if (this.disposed)
            return;
        this.generation++;
        this.pending = false;
        this.error = null;
        this.diagnostics = [];
        this.clearArtifact();
        setUiText(this.status, reason);
        this.showState();
    }
    dispose(): void {
        if (this.disposed)
            return;
        this.disposed = true;
        this.generation++;
        this.pending = false;
        this.clearArtifact();
        this.element.remove();
    }
    private clearArtifact(): void {
        // Hide the native viewer too: its own print/download controls must not
        // continue to look like actions on a newer Draft.
        this.frame.hidden = true;
        this.frame.removeAttribute("src");
        if (this.artifact)
            URL.revokeObjectURL(this.artifact.url);
        this.artifact = null;
        this.download.removeAttribute("href");
        this.open.removeAttribute("href");
        setUiText(this.help, "");
    }
    private guardAction(event: Event): boolean {
        if (!this.state.current) {
            event.preventDefault();
            this.invalidate();
            return false;
        }
        return true;
    }
    private showState(): void {
        const state = this.state;
        this.element.classList.toggle("paged-pending", state.pending);
        setUiText(this.freshness, state.current
            ? displayMessage("pagedPreview.draftPageSummary", { p0: state.pageCount }) : state.pending ? uiMessage("pagedPreview.compilingCurrentDraft") : uiMessage("pagedPreview.previewNotReady"));
        this.freshness.classList.toggle("current", state.current);
        for (const action of [this.download, this.open]) {
            action.setAttribute("aria-disabled", String(!state.current));
            action.tabIndex = state.current ? 0 : -1;
        }
        this.status.hidden = !this.status.textContent;
        this.help.hidden = !this.help.textContent;
        this.diagnosticDetails.replaceChildren();
        this.diagnosticDetails.hidden = !state.diagnostics.length;
        if (state.diagnostics.length) {
            const errors = state.diagnostics.filter(item => item.severity === "error").length;
            this.diagnosticDetails.open = errors > 0;
            const summary = document.createElement("summary");
            setUiText(summary, () => `${uiTextValue(errors ? displayMessage("shell.errorCount", { p0: errors }) : displayMessage("shell.noticeCount", { p0: state.diagnostics.length }))} · Typst Paged 0.15.1`);
            this.diagnosticDetails.append(summary);
            for (const diagnostic of state.diagnostics) {
                const item = document.createElement("p");
                item.className = `paged-diagnostic ${diagnostic.severity}`;
                setUiText(item, engineDiagnosticUiText(diagnostic));
                this.diagnosticDetails.append(item);
            }
        }
        this.options.onState?.(state);
    }
}
import { engineDiagnosticUiText } from "./ui-diagnostics";
