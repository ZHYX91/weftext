import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { MediaResource } from "./media-viewer";
import { decodeThumbnailResponse, type ThumbnailResponse } from "./media-wire";
import { descriptionUiText } from "./ui-diagnostics";
export type ThumbnailState = "absent" | "building" | "ready" | "failed" | "unsupported" | "unavailable";
/** The server never reports client building/failed states. */
export type ListPreview = ThumbnailResponse;
interface ThumbnailDisplay {
    state: ThumbnailState;
    message?: UiText;
}
const labels: Record<ThumbnailState, UiText> = { absent: uiMessage("thumbnail.absent"), building: uiMessage("thumbnail.building"), ready: uiMessage("thumbnail.ready"), failed: uiMessage("thumbnail.failed"), unsupported: uiMessage("thumbnail.unsupported"), unavailable: uiMessage("thumbnail.unavailable") };
/** A display projection must name its exact source and resource basis. */
export function thumbnailSourceBasis(preview: Pick<ListPreview, "sourceBasisKind" | "sourceBaseId" | "sourceRevision">): string {
    if (preview.sourceBasisKind === "observed" && preview.sourceBaseId && preview.sourceRevision === null)
        return `observed-source:${preview.sourceBaseId}`;
    if (preview.sourceBasisKind === "committed" && preview.sourceRevision && preview.sourceBaseId === null)
        return preview.sourceRevision;
    throw new UiError(uiMessage("thumbnail.sourceBasisMissing"));
}
export function thumbnailStatus(preview: ThumbnailDisplay): UiText {
    if (!(preview.state in labels))
        throw new UiError(uiMessage("thumbnail.unknownState"));
    return () => uiTextValue(labels[preview.state]) + (preview.message ? `：${uiTextValue(preview.message)}` : "");
}
export interface ListThumbnailHost {
    resolve(): Promise<unknown>;
    read(descriptor: MediaResource, sourceBasis: string): Promise<Blob>;
    viewport?: HTMLElement;
}
/** Each visible row loads independently. Disposal clears pixels and suppresses
 * late completions; neither focus nor editor/session state is touched. */
export function mountListThumbnail(container: HTMLElement, host: ListThumbnailHost): {
    dispose(): void;
} {
    let disposed = false, started = false;
    let renderer: {
        dispose(): void;
    } | undefined;
    const status = document.createElement("p");
    status.className = "query-preview-state";
    setUiText(status, uiMessage("thumbnail.waitingVisible"));
    container.dataset.previewState = "building";
    container.replaceChildren(status);
    const state = (preview: ThumbnailDisplay) => { container.dataset.previewState = preview.state; setUiText(status, thumbnailStatus(preview)); };
    const build = async () => {
        if (disposed || started || !container.isConnected)
            return;
        started = true;
        state({ state: "building" });
        try {
            const preview = decodeThumbnailResponse(await host.resolve());
            if (disposed || !container.isConnected)
                return;
            state({ state: preview.state, ...(preview.state !== "ready" ? { message: descriptionUiText(preview) } : {}) });
            if (preview.state !== "ready")
                return;
            const descriptor = preview.descriptor;
            if (!descriptor?.revision || !["image", "pdf"].includes(descriptor.metadata.kind))
                throw new UiError(uiMessage("thumbnail.resourceBasisMissing"));
            const basis = thumbnailSourceBasis(preview), media = await import("./media-viewer");
            if (disposed || !container.isConnected)
                return;
            const pixels = document.createElement("div");
            pixels.className = "query-thumbnail-pixels";
            container.replaceChildren(pixels, status);
            state({ state: "building" });
            renderer = media.renderQueryThumbnail(pixels, descriptor, () => host.read(descriptor, basis), outcome => {
                if (!disposed)
                    state({ state: outcome, message: outcome === "ready" ? displayMessage("thumbnail.exactResourceVersion", { p0: uiDisplayParameter(descriptor.revision) }) : uiMessage("thumbnail.decodeIncomplete") });
            });
        }
        catch (error) {
            if (!disposed && container.isConnected)
                state({ state: "failed", message: errorUiText(error) });
        }
    };
    const observer = typeof IntersectionObserver === "undefined" ? undefined : new IntersectionObserver(entries => {
        if (entries.some(entry => entry.isIntersecting)) {
            observer!.disconnect();
            void build();
        }
    }, { root: host.viewport ?? null, rootMargin: "120px" });
    if (observer)
        observer.observe(container);
    else
        queueMicrotask(() => { void build(); });
    return { dispose() { disposed = true; observer?.disconnect(); renderer?.dispose(); container.replaceChildren(); } };
}
