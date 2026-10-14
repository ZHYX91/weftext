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
  dependencies: { path: string; kind: string; status?: string }[];
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
    || !/^[A-Za-z0-9+/]+={0,2}$/.test(base64)) throw new Error("分页输出没有有效的 PDF 数据。");
  let decoded: string;
  try { decoded = atob(base64); } catch { throw new Error("PDF 数据无法解码。"); }
  if (decoded.length > PDF_BYTES_LIMIT || !decoded.startsWith("%PDF-")) throw new Error("分页输出不是受支持的 PDF，或超过 16 MiB 上限。");
  const bytes = new Uint8Array(decoded.length);
  for (let index = 0; index < decoded.length; index++) bytes[index] = decoded.charCodeAt(index);
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
  private artifact: { basis: PagedBasis; output: PagedOutput; url: string } | null = null;
  private diagnostics: Diagnostic[] = [];
  private error: string | null = null;

  constructor(host: HTMLElement, private readonly options: PagedPreviewOptions) {
    this.element = document.createElement("section");
    this.element.className = "paged-preview";
    this.element.setAttribute("aria-label", "当前草稿的分页 PDF 预览");
    const toolbar = document.createElement("div"); toolbar.className = "paged-toolbar";
    this.freshness = document.createElement("span"); this.freshness.className = "paged-freshness";
    this.freshness.setAttribute("role", "status");
    const actions = document.createElement("div"); actions.className = "paged-actions";
    this.download = document.createElement("a"); this.download.className = "button-secondary compact";
    this.download.textContent = "下载 PDF";
    this.open = document.createElement("a"); this.open.className = "button-secondary compact";
    this.open.textContent = "打开 PDF / 打印…"; this.open.target = "_blank"; this.open.rel = "noopener noreferrer";
    this.open.title = "打开当前草稿的 PDF，使用查看器打印或按 Ctrl+P";
    this.download.addEventListener("click", event => this.guardAction(event));
    this.open.addEventListener("click", event => {
      if (this.guardAction(event)) {
        this.help.textContent = "在打开的 PDF 查看器中点击打印，或按 Ctrl+P。打印内容基于此时的草稿。";
      }
    });
    actions.append(this.download, this.open); toolbar.append(this.freshness, actions);
    this.status = document.createElement("div"); this.status.className = "paged-status";
    this.status.setAttribute("role", "status"); this.status.setAttribute("aria-live", "polite");
    this.frame = document.createElement("iframe"); this.frame.className = "paged-viewer";
    this.frame.title = "Typst 原生分页 PDF：当前草稿（只读）";
    this.frame.setAttribute("referrerpolicy", "no-referrer");
    this.help = document.createElement("p"); this.help.className = "paged-help";
    this.diagnosticDetails = document.createElement("details"); this.diagnosticDetails.className = "paged-diagnostics";
    this.element.append(toolbar, this.status, this.frame, this.help, this.diagnosticDetails); host.append(this.element);
    this.clearArtifact(); this.showState();
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
    if (this.disposed) return;
    const capture = proposedBasis ?? this.options.basis();
    if (!capture || !samePagedBasis(capture, this.options.basis())) return;
    const basis = { ...capture };
    const generation = ++this.generation;
    this.clearArtifact(); this.pending = true; this.error = null; this.diagnostics = [];
    this.status.textContent = "正在使用官方 Typst Paged 编译当前草稿…"; this.showState();
    try {
      const output = await this.options.request({ ...basis });
      if (this.disposed || generation !== this.generation) return;
      if (!samePagedBasis(basis, this.options.basis())) {
        this.status.textContent = "草稿已更新，等待当前草稿的分页结果。";
        return;
      }
      if (!pagedResultMatches(basis, output)) throw new Error("分页结果已过期，正在等待当前草稿与样式的结果。");
      this.diagnostics = output.diagnostics;
      if (!output.ok || !output.pdfBase64) {
        const detail = output.diagnostics.find(item => item.severity === "error")?.message;
        throw new Error(detail ? `分页预览不可用：${detail}` : "当前草稿暂时无法生成 PDF。请查看源码与编译诊断。");
      }
      if (!Number.isSafeInteger(output.pageCount) || output.pageCount < 1 || output.pageCount > 1000) throw new Error("分页结果没有有效的页数。");
      const bytes = decodePagedPdf(output.pdfBase64);
      const url = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
      this.artifact = { basis: { ...basis }, output, url };
      this.frame.src = `${url}#view=FitH`;
      this.frame.hidden = false;
      this.frame.title = `Typst 原生分页 PDF：当前草稿，第 ${basis.serial} 次修改，共 ${output.pageCount} 页（只读）`;
      this.download.href = url; this.download.download = pagedFileName(this.options.fileName?.() ?? "草稿.typ");
      this.open.href = url;
      this.help.textContent = "可使用 PDF 查看器的页码、缩放、选择、复制与打印控件。分页预览只读，源内容仍在同一草稿中。";
      this.status.textContent = "";
    } catch (error) {
      if (!this.disposed && generation === this.generation && samePagedBasis(basis, this.options.basis())) {
        this.error = error instanceof Error ? error.message : String(error);
        this.status.textContent = this.error;
        this.options.onError?.(error instanceof Error ? error : new Error(this.error));
      }
    } finally {
      if (!this.disposed && generation === this.generation) {
        this.pending = false; this.showState();
      }
    }
  }

  /** Also call on style, dependency-root, session, or environment changes. */
  invalidate(reason = "草稿或样式已变化，分页预览需要重新编译。"): void {
    if (this.disposed) return;
    this.generation++; this.pending = false; this.error = null; this.diagnostics = [];
    this.clearArtifact(); this.status.textContent = reason; this.showState();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true; this.generation++; this.pending = false;
    this.clearArtifact(); this.element.remove();
  }

  private clearArtifact(): void {
    // Hide the native viewer too: its own print/download controls must not
    // continue to look like actions on a newer Draft.
    this.frame.hidden = true; this.frame.removeAttribute("src");
    if (this.artifact) URL.revokeObjectURL(this.artifact.url);
    this.artifact = null;
    this.download.removeAttribute("href"); this.open.removeAttribute("href");
    this.help.textContent = "";
  }

  private guardAction(event: Event): boolean {
    if (!this.state.current) {
      event.preventDefault(); this.invalidate();
      return false;
    }
    return true;
  }

  private showState(): void {
    const state = this.state;
    this.element.classList.toggle("paged-pending", state.pending);
    this.freshness.textContent = state.current
      ? `当前草稿 · ${state.pageCount} 页 · Typst Paged 0.15.1`
      : state.pending ? "正在编译当前草稿" : "分页预览尚不可用";
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
      summary.textContent = `${errors ? `${errors} 个错误` : `${state.diagnostics.length} 项提示`} · Typst Paged 0.15.1`;
      this.diagnosticDetails.append(summary);
      for (const diagnostic of state.diagnostics) {
        const item = document.createElement("p"); item.className = `paged-diagnostic ${diagnostic.severity}`;
        item.textContent = diagnostic.message;
        this.diagnosticDetails.append(item);
      }
    }
    this.options.onState?.(state);
  }
}
