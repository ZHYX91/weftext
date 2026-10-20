import type { MediaResource } from "./media-viewer";

export type ThumbnailState = "absent" | "building" | "ready" | "failed" | "unsupported" | "unavailable";
export interface ListPreview {
  state: ThumbnailState;
  descriptor?: MediaResource;
  sourceRevision?: string;
  sourceBaseId?: string;
  sourceBasisKind?: "observed" | "committed";
  message?: string;
}
const labels: Record<ThumbnailState, string> = { absent: "没有图片或 PDF 资源", building: "缩略图生成中", ready: "缩略图就绪", failed: "缩略图生成失败", unsupported: "此格式暂无缩略图", unavailable: "缩略图不可用" };

/** A display projection must name its exact source and resource basis. */
export function thumbnailSourceBasis(preview: ListPreview): string {
  if (preview.sourceBasisKind === "observed" && preview.sourceBaseId) return `observed-source:${preview.sourceBaseId}`;
  if (preview.sourceBasisKind === "committed" && preview.sourceRevision) return preview.sourceRevision;
  throw new Error("缩略图缺少准确笔记读取依据");
}
export function thumbnailStatus(preview: ListPreview): string {
  if (!(preview.state in labels)) throw new Error("缩略图返回未知状态");
  return `${labels[preview.state]}${preview.message ? `：${preview.message}` : ""}`;
}
export interface ListThumbnailHost {
  resolve(): Promise<ListPreview>;
  read(descriptor: MediaResource, sourceBasis: string): Promise<Blob>;
  viewport?: HTMLElement;
}

/** Each visible row loads independently. Disposal clears pixels and suppresses
 * late completions; neither focus nor editor/session state is touched. */
export function mountListThumbnail(container: HTMLElement, host: ListThumbnailHost): { dispose(): void } {
  let disposed = false, started = false;
  let renderer: { dispose(): void } | undefined;
  const status = document.createElement("p"); status.className = "query-preview-state";
  status.textContent = "缩略图等待可见时读取…"; container.dataset.previewState = "building"; container.replaceChildren(status);
  const state = (preview: ListPreview) => { container.dataset.previewState = preview.state; status.textContent = thumbnailStatus(preview); };
  const build = async () => {
    if (disposed || started || !container.isConnected) return; started = true; state({ state: "building" });
    try {
      const preview = await host.resolve(); if (disposed || !container.isConnected) return;
      state(preview); if (preview.state !== "ready") return;
      const descriptor = preview.descriptor;
      if (!descriptor?.revision || !["image", "pdf"].includes(descriptor.metadata.kind)) throw new Error("缩略图缺少准确图片或 PDF 资源");
      const basis = thumbnailSourceBasis(preview), media = await import("./media-viewer"); if (disposed || !container.isConnected) return;
      const pixels = document.createElement("div"); pixels.className = "query-thumbnail-pixels";
      container.replaceChildren(pixels, status); state({ state: "building" });
      renderer = media.renderQueryThumbnail(pixels, descriptor, () => host.read(descriptor, basis), outcome => {
        if (!disposed) state({ state: outcome, message: outcome === "ready" ? `准确资源版本 ${descriptor.revision}` : "资源解码未完成，原件仍可明确打开查看。" });
      });
    } catch (error) { if (!disposed && container.isConnected) state({ state: "unavailable", message: error instanceof Error ? error.message : String(error) }); }
  };
  const observer = typeof IntersectionObserver === "undefined" ? undefined : new IntersectionObserver(entries => {
    if (entries.some(entry => entry.isIntersecting)) { observer!.disconnect(); void build(); }
  }, { root: host.viewport ?? null, rootMargin: "120px" });
  if (observer) observer.observe(container); else queueMicrotask(() => { void build(); });
  return { dispose() { disposed = true; observer?.disconnect(); renderer?.dispose(); container.replaceChildren(); } };
}
