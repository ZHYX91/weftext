import { disposeUiTree, joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./media-viewer.css";
import type { MediaRegionInput } from "./knowledge-state";
import { validateMediaRegion } from "./knowledge-state";
import { mediaRectangle, playerTick, resourceOpenError } from "./media-selection";
import type { MediaPoint, ResourceOpenTarget } from "./media-selection";
import type { ErrorDescription } from "./ui-diagnostics";
import { descriptionUiText } from "./ui-diagnostics";
import type { MediaPage, ResourceMetadata } from "./media-wire";
export type { MediaPage, ResourceMetadata } from "./media-wire";
import { getDocument, GlobalWorkerOptions } from "pdfjs-dist";
import type { PDFDocumentProxy, PDFDocumentLoadingTask, RenderTask } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
GlobalWorkerOptions.workerSrc = workerUrl;
export interface MediaResource {
    resourceId: string;
    resource: string;
    nodeId?: string;
    name: string;
    revision: string | null;
    readBasis?: string;
    bytes: number | null;
    contentType: string;
    metadata: ResourceMetadata;
    annotationReady: boolean;
    currentness: string;
}
export interface ResourceList {
    resources: MediaResource[];
    warnings: readonly ErrorDescription[];
}
export interface MediaViewerHost {
    context(): {
        sessionId?: string;
        viewerId?: string;
    } | null;
    list(sessionId: string): Promise<ResourceList>;
    read(sessionId: string, resource: string, revision: string): Promise<Blob>;
    importResource?(file: File, sessionId: string): Promise<void>;
    notify(message: UiText, error?: boolean): void;
}
export interface MediaViewer {
    dispose(): void;
    selection(): MediaRegionInput | null;
}
export interface MediaViewerOptions {
    readOnly?: boolean;
    open?: ResourceOpenTarget;
    allowSaveCopy?: boolean;
    fixedResource?: boolean;
}
/** Open a resolved resource in an independent read-only panel/dialog. */
export function mountResourceViewer(container: HTMLElement, host: MediaViewerHost, target: ResourceOpenTarget, options: Pick<MediaViewerOptions, "allowSaveCopy"> = {}): {
    dispose(): void;
} {
    return mountMediaViewer(container, host, () => { }, undefined, { ...options, readOnly: true, open: target });
}
/** An opt-in list preview reads the exact resource version independently of
 * opening a note. It does not change the editor, selection, or query ordering. */
export function renderQueryThumbnail(container: HTMLElement, descriptor: MediaResource, read: () => Promise<Blob>, report?: (state: "ready" | "failed") => void): {
    dispose(): void;
} {
    let disposed = false, objectUrl: string | null = null, loading: PDFDocumentLoadingTask | null = null, rendering: RenderTask | null = null;
    container.classList.add("media-thumbnail");
    setUiAttr(container, "aria-label", displayMessage("media.exactThumbnailAria", { p0: uiDisplayParameter(descriptor.name) }));
    setUiText(container, uiMessage("media.generatingPreview"));
    void (async () => {
        try {
            const blob = await read();
            if (disposed)
                return;
            if (!descriptor.revision || blob.size !== descriptor.bytes || blob.type !== descriptor.contentType)
                throw new UiError(uiMessage("media.thumbnailDeliveryMismatch"));
            if (descriptor.metadata.kind === "image") {
                const image = el("img", "media-thumbnail-image");
                image.alt = descriptor.name;
                objectUrl = URL.createObjectURL(blob);
                image.src = objectUrl;
                await image.decode();
                if (disposed)
                    return;
                if (image.naturalWidth !== descriptor.metadata.width || image.naturalHeight !== descriptor.metadata.height)
                    throw new UiError(uiMessage("media.thumbnailImageDimensionsMismatch"));
                container.replaceChildren(image);
            }
            else if (descriptor.metadata.kind === "pdf") {
                const data = new Uint8Array(await blob.arrayBuffer());
                if (disposed)
                    return;
                const task = getDocument({ data, useSystemFonts: false, disableAutoFetch: true, useWasm: false });
                loading = task;
                const document = await task.promise;
                if (disposed)
                    return;
                if (document.numPages !== descriptor.metadata.pages.length)
                    throw new UiError(uiMessage("media.thumbnailPdfPageCountMismatch"));
                const page = await document.getPage(1);
                if (disposed)
                    return;
                const facts = descriptor.metadata.pages[0], unrotated = page.getViewport({ scale: 1, rotation: 0 });
                if (!facts || Math.abs(unrotated.width - facts.width) > 0.001 || Math.abs(unrotated.height - facts.height) > 0.001 || page.rotate !== facts.rotation || page.userUnit !== 1)
                    throw new UiError(uiMessage("media.thumbnailPdfCoordinatesMismatch"));
                const view = page.getViewport({ scale: 1 }), viewport = page.getViewport({ scale: 160 / Math.max(view.width, view.height) });
                const canvas = el("canvas", "media-thumbnail-image");
                canvas.width = Math.ceil(viewport.width);
                canvas.height = Math.ceil(viewport.height);
                setUiAttr(canvas, "aria-label", displayMessage("media.firstPageAlt", { p0: uiDisplayParameter(descriptor.name) }));
                const renderTask = page.render({ canvas, viewport, annotationMode: 0 });
                rendering = renderTask;
                await renderTask.promise;
                rendering = null;
                if (disposed)
                    return;
                container.replaceChildren(canvas);
                await document.cleanup();
            }
            else
                throw new UiError(uiMessage("media.thumbnailUnsupported"));
            if (!disposed)
                report?.("ready");
        }
        catch (error) {
            if (!disposed) {
                setUiText(container, error instanceof Error ? errorUiText(error) : uiMessage("media.previewFailed"));
                container.classList.add("media-thumbnail-error");
                report?.("failed");
            }
        }
    })();
    return { dispose: () => {
            disposed = true;
            rendering?.cancel();
            if (loading)
                void loading.destroy();
            if (objectUrl)
                URL.revokeObjectURL(objectUrl);
            container.replaceChildren();
        } };
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: UiText): HTMLElementTagNameMap[K] {
    const element = document.createElement(tag);
    element.className = className;
    if (text !== undefined)
        setUiText(element, text);
    return element;
}
function button(label: UiText, callback: () => void | Promise<void>): HTMLButtonElement {
    const control = el("button", "media-button", label);
    control.type = "button";
    control.onclick = () => { void Promise.resolve(callback()).catch(() => { }); };
    return control;
}
function labeled(label: UiText, control: HTMLElement): HTMLLabelElement { const wrapper = el("label", "media-field"); wrapper.append(el("span", "", label), control); return wrapper; }
/** The API never receives editable resource identity, dimensions, or timebase. */
export function mountMediaViewer(container: HTMLElement, host: MediaViewerHost, selected: (target: MediaRegionInput | null) => void, initial?: MediaRegionInput, options: MediaViewerOptions = {}): MediaViewer {
    container.classList.add("media-viewer");
    const sessionId = host.context()?.sessionId;
    const viewerId = host.context()?.viewerId;
    const lifetime = viewerId ?? sessionId;
    let disposed = false, generation = 0, pageGeneration = 0, resource: MediaResource | null = null, current: MediaRegionInput | null = null;
    let remembered = initial ? { ...initial } : undefined;
    let objectUrl: string | null = null, pdf: PDFDocumentProxy | null = null, loading: PDFDocumentLoadingTask | null = null, rendering: RenderTask | null = null;
    let player: HTMLMediaElement | null = null;
    let decodingImage: HTMLImageElement | null = null;
    const cancelDecoding = new Set<() => void>();
    const toolbar = el("div", "media-toolbar"), choose = el("select", "media-input"), status = el("p", "media-status", uiMessage("media.loadingResourceList"));
    const controls = el("div", "media-controls"), preview = el("div", "media-preview"), details = el("p", "media-details");
    const saveCopy = options.readOnly && options.allowSaveCopy !== false ? button(uiMessage("media.saveExactCopy"), () => {
        if (!objectUrl || !resource)
            return;
        const download = el("a");
        download.href = objectUrl;
        download.download = resource.name;
        download.click();
    }) : null;
    setUiAttr(choose, "aria-label", uiMessage("media.registeredResourceAria"));
    toolbar.append(choose, button(uiMessage("media.refreshResources"), () => refresh()));
    if (saveCopy) {
        saveCopy.disabled = true;
        toolbar.append(saveCopy);
    }
    choose.hidden = !!options.readOnly || !!options.fixedResource;
    container.append(toolbar, status, controls, preview, details);
    if (host.importResource && sessionId && !options.readOnly) {
        const picker = el("input", "media-file");
        picker.type = "file";
        setUiAttr(picker, "aria-label", uiMessage("media.importFileAria"));
        picker.hidden = true;
        picker.onchange = () => {
            const file = picker.files?.[0];
            picker.value = "";
            if (!file)
                return;
            if (file.size > 64 * 1024 * 1024) {
                report(uiMessage("media.importBudgetExceeded"), true);
                return;
            }
            void host.importResource!(file, sessionId).then(refresh).catch(error => report(message(error), true));
        };
        toolbar.append(button(uiMessage("media.importResource"), () => picker.click()), picker);
    }
    function message(error: unknown): UiText { return error instanceof Error ? errorUiText(error) : uiMessage("media.resourceReadIncomplete"); }
    function report(text: UiText, error = false): void {
        setUiText(status, text);
        status.classList.toggle("error", error);
        if (error)
            host.notify(text, true);
    }
    function currentLifetime(): string | undefined { const context = host.context(); return context?.viewerId ?? context?.sessionId; }
    function active(ticket: number): boolean { return !disposed && generation === ticket && currentLifetime() === lifetime; }
    function set(value: MediaRegionInput | null): void {
        current = options.readOnly ? null : value;
        if (current)
            remembered = { ...current };
        selected(current && validateMediaRegion(current) === null ? { ...current } : null);
        updateDetails();
    }
    function updateDetails(): void {
        if (!current) {
            setUiText(details, options.readOnly ? uiMessage("media.readOnlyExactVersion") : uiMessage("media.chooseVersionAndTarget"));
            return;
        }
        const rect = current.rectWidth === undefined ? "" : displayMessage("media.rectangleSummary", { p0: uiDisplayParameter(current.x), p1: uiDisplayParameter(current.y), p2: uiDisplayParameter(current.rectWidth), p3: uiDisplayParameter(current.rectHeight), p4: uiDisplayParameter(current.width), p5: uiDisplayParameter(current.height), p6: uiDisplayParameter(current.kind === "pdf_region" ? uiMessage("media.pdfOriginalOriginUnits") : uiMessage("media.originalPixels")) });
        const interval = current.kind === "media_interval" ? displayMessage("media.intervalSummary", { p0: uiDisplayParameter(current.startTick), p1: uiDisplayParameter(current.endTick), p2: uiDisplayParameter(current.timebaseDenominator ?? 1), p3: uiDisplayParameter(current.timebase), p4: uiDisplayParameter(current.durationTick) }) : "";
        setUiText(details, joinUiText([current.kind === "pdf_region" ? displayMessage("media.pageLabel", { p0: uiDisplayParameter((current.pageIndex ?? 0) + 1) }) : "", rect, interval].filter(Boolean), "；"));
        const error = validateMediaRegion(current);
        if (error)
            setUiText(details, joinUiText([current.kind === "pdf_region" ? displayMessage("media.pageLabel", { p0: uiDisplayParameter((current.pageIndex ?? 0) + 1) }) : "", rect, interval, error].filter(Boolean), "；"));
    }
    function clean(): void {
        for (const cancel of cancelDecoding)
            cancel();
        cancelDecoding.clear();
        if (decodingImage) {
            decodingImage.removeAttribute("src");
            decodingImage = null;
        }
        if (saveCopy)
            saveCopy.disabled = true;
        pageGeneration++;
        rendering?.cancel();
        rendering = null;
        if (loading) {
            void loading.destroy();
            loading = null;
        }
        else if (pdf) {
            void pdf.loadingTask.destroy();
        }
        pdf = null;
        if (player) {
            player.pause();
            player.removeAttribute("src");
            player.load();
            player = null;
        }
        if (objectUrl)
            URL.revokeObjectURL(objectUrl);
        objectUrl = null;
        disposeUiTree(controls);
        disposeUiTree(preview);
        controls.replaceChildren();
        preview.replaceChildren();
    }
    async function refresh(): Promise<void> {
        if (!lifetime || currentLifetime() !== lifetime) {
            report(uiMessage("media.viewAuthorizationChanged"));
            return;
        }
        const ticket = ++generation;
        set(null);
        clean();
        choose.disabled = true;
        try {
            const result = await host.list(lifetime);
            if (!active(ticket))
                return;
            choose.replaceChildren();
            const empty = el("option", "", uiMessage("media.chooseNoteResource"));
            empty.value = "";
            choose.append(empty);
            for (const item of result.resources) {
                const option = el("option", "", () => `${uiTextValue(item.name)}${uiTextValue(item.revision ? "" : uiMessage("media.versionUnadmittedSuffix"))}`);
                option.value = item.resource;
                option.disabled = !item.revision;
                choose.append(option);
            }
            choose.disabled = false;
            report(result.warnings.length ? joinUiText(result.warnings.map(descriptionUiText), "；") : result.resources.length ? uiMessage("media.chooseResourceToRead") : uiMessage("media.noRegisteredResources"));
            choose.onchange = () => {
                const item = result.resources.find(item => item.resource === choose.value);
                if (item)
                    void open(item);
                else {
                    ++generation;
                    clean();
                    set(null);
                }
            };
            if (options.open) {
                const target = result.resources.find(item => item.resource === options.open!.resource);
                const error = target ? resourceOpenError(options.open, target) : uiMessage("media.resourceOutsideAuthorizedManifest");
                if (error || !target) {
                    report(error!, true);
                    return;
                }
                choose.value = target.resource;
                await open(target);
                return;
            }
            const previous = remembered && result.resources.find(item => item.resource === remembered!.resource && item.revision === remembered!.revision);
            if (previous) {
                choose.value = previous.resource;
                await open(previous);
            }
        }
        catch (error) {
            if (active(ticket)) {
                choose.disabled = false;
                report(message(error), true);
            }
        }
    }
    async function open(item: MediaResource): Promise<void> {
        const ticket = ++generation;
        clean();
        set(null);
        resource = item;
        if (!lifetime || !item.revision) {
            report(uiMessage("media.exactVersionMissing"));
            return;
        }
        report(uiMessage("media.loadingExactVersion"));
        try {
            const blob = await host.read(lifetime, item.resource, item.revision);
            if (!active(ticket))
                return;
            if (blob.size !== item.bytes || blob.type !== item.contentType)
                throw new UiError(uiMessage("media.deliveryManifestMismatch"));
            objectUrl = URL.createObjectURL(blob);
            if (saveCopy)
                saveCopy.disabled = false;
            switch (item.metadata.kind) {
                case "image":
                    await showImage(item, ticket);
                    break;
                case "pdf":
                    await showPdf(item, blob, ticket);
                    break;
                case "audio":
                case "video":
                    await showPlayer(item, ticket);
                    break;
                case "unsupported":
                    await showUnsupported(item, blob, ticket);
                    break;
            }
            if (active(ticket))
                report(item.metadata.kind === "unsupported" ? descriptionUiText(item.metadata) : displayMessage("media.exactVersionOpened", { p0: uiDisplayParameter(item.name) }));
        }
        catch (error) {
            if (active(ticket)) {
                clean();
                set(null);
                report(message(error), true);
            }
        }
    }
    function base(kind: MediaRegionInput["kind"]): MediaRegionInput { return { kind, resource: resource!.resource, revision: resource!.revision! }; }
    function restore(target: MediaRegionInput): MediaRegionInput {
        if (!remembered || remembered.resource !== target.resource || remembered.revision !== target.revision || remembered.kind !== target.kind || remembered.pageIndex !== target.pageIndex)
            return target;
        return { ...target, x: remembered.x, y: remembered.y, rectWidth: remembered.rectWidth, rectHeight: remembered.rectHeight,
            ...(target.kind === "media_interval" ? { startTick: remembered.startTick, endTick: remembered.endTick } : {}) };
    }
    function drawSurface(surface: HTMLImageElement | HTMLCanvasElement | HTMLVideoElement, width: number, height: number, pixels: boolean): void {
        const stage = el("div", "media-stage"), outline = el("div", "media-rectangle");
        surface.classList.add("media-surface");
        stage.style.aspectRatio = `${width} / ${height}`;
        stage.append(surface, outline);
        preview.append(stage);
        outline.hidden = true;
        let start: MediaPoint | null = null;
        if (options.readOnly) {
            stage.classList.add("media-stage-readonly");
            setUiText(details, displayMessage("media.surfaceSummary", { p0: uiDisplayParameter(width), p1: uiDisplayParameter(height), p2: uiDisplayParameter(pixels ? uiMessage("media.originalPixels") : uiMessage("media.pdfUnrotatedUnits")), p3: uiDisplayParameter(resource!.revision) }));
            return;
        }
        const point = (event: PointerEvent): MediaPoint => { const box = surface.getBoundingClientRect(); return { x: (event.clientX - box.left) / box.width * width, y: (event.clientY - box.top) / box.height * height }; };
        const paint = (): void => {
            if (!current?.rectWidth) {
                outline.hidden = true;
                return;
            }
            outline.hidden = false;
            outline.style.left = `${100 * current.x! / width}%`;
            outline.style.top = `${100 * current.y! / height}%`;
            outline.style.width = `${100 * current.rectWidth / width}%`;
            outline.style.height = `${100 * current.rectHeight! / height}%`;
        };
        surface.addEventListener("pointerdown", raw => {
            const event = raw as PointerEvent;
            if (event.button !== 0 || !current)
                return;
            event.preventDefault();
            start = point(event);
            surface.setPointerCapture(event.pointerId);
        });
        surface.addEventListener("pointermove", raw => {
            const event = raw as PointerEvent;
            if (!start || !current)
                return;
            const rect = mediaRectangle(start, point(event), width, height, pixels);
            if (rect) {
                set({ ...current, ...rect });
                paint();
            }
        });
        surface.addEventListener("pointerup", raw => {
            const event = raw as PointerEvent;
            if (!start || !current)
                return;
            const rect = mediaRectangle(start, point(event), width, height, pixels);
            start = null;
            if (surface.hasPointerCapture(event.pointerId))
                surface.releasePointerCapture(event.pointerId);
            if (rect) {
                set({ ...current, ...rect });
                paint();
            }
        });
        surface.addEventListener("pointercancel", () => { start = null; });
        const inputGroup = el("div", "media-coordinate-fields");
        for (const [key, label] of [["x", uiMessage("media.rectangleLeft")], ["y", uiMessage("media.rectangleTop")], ["rectWidth", uiMessage("media.rectangleWidth")], ["rectHeight", uiMessage("media.rectangleHeight")]] as const) {
            const input = el("input", "media-input");
            input.type = "number";
            input.min = "0";
            input.step = pixels ? "1" : "any";
            input.value = current?.[key]?.toString() ?? "";
            input.oninput = () => {
                if (!current)
                    return;
                const value = input.value === "" ? undefined : Number(input.value);
                set({ ...current, [key]: value });
                paint();
            };
            inputGroup.append(labeled(label, input));
        }
        controls.append(inputGroup, el("p", "media-help", displayMessage("media.rectangleSelectionHelp", { p0: uiDisplayParameter(pixels ? uiMessage("media.pixelPreview") : uiMessage("media.unrotatedPdfPage")) })));
        // Coordinate fields reflect drag updates without becoming dimension authority.
        stage.addEventListener("pointerup", () => { const inputs = inputGroup.querySelectorAll("input"); ["x", "y", "rectWidth", "rectHeight"].forEach((key, index) => { inputs[index].value = String(current?.[key as keyof MediaRegionInput] ?? ""); }); });
        paint();
    }
    async function showImage(item: MediaResource, ticket: number): Promise<void> {
        if (item.metadata.kind !== "image")
            return;
        const image = el("img");
        decodingImage = image;
        image.alt = item.name;
        image.draggable = false;
        image.src = objectUrl!;
        try {
            await image.decode();
        }
        finally {
            if (decodingImage === image)
                decodingImage = null;
        }
        if (!active(ticket))
            return;
        const { width, height } = item.metadata;
        if (image.naturalWidth !== width || image.naturalHeight !== height)
            throw new UiError(uiMessage("media.imageMetadataMismatch"));
        set(restore({ ...base("image_region"), width, height }));
        drawSurface(image, width, height, true);
    }
    async function showPdf(item: MediaResource, blob: Blob, ticket: number): Promise<void> {
        if (item.metadata.kind !== "pdf")
            return;
        const data = new Uint8Array(await blob.arrayBuffer());
        if (!active(ticket))
            return;
        const task = getDocument({ data, useSystemFonts: false, disableAutoFetch: true, useWasm: false });
        loading = task;
        const loaded = await task.promise;
        if (!active(ticket)) {
            void loaded.loadingTask.destroy();
            return;
        }
        pdf = loaded;
        if (loaded.numPages !== item.metadata.pages.length)
            throw new UiError(uiMessage("media.pdfPageCountMismatch"));
        const pageSelect = el("select", "media-input");
        for (let i = 0; i < loaded.numPages; i++) {
            const option = el("option", "", displayMessage("media.pageLabel", { p0: uiDisplayParameter(i + 1) }));
            option.value = String(i);
            pageSelect.append(option);
        }
        const pageField = labeled(uiMessage("media.pdfPageAria"), pageSelect);
        controls.append(pageField);
        const renderPage = async (index: number): Promise<void> => {
            const pageTicket = ++pageGeneration;
            rendering?.cancel();
            preview.replaceChildren();
            controls.replaceChildren(pageField);
            set(null);
            const page = await loaded.getPage(index + 1);
            if (!active(ticket) || pageTicket !== pageGeneration || item.metadata.kind !== "pdf")
                return;
            const facts = item.metadata.pages[index], view = page.getViewport({ scale: 1, rotation: 0 });
            if (Math.abs(view.width - facts.width) > 0.001 || Math.abs(view.height - facts.height) > 0.001 || page.rotate !== facts.rotation || page.userUnit !== 1)
                throw new UiError(uiMessage("media.pdfGeometryMismatch"));
            const scale = Math.min(2, 1000 / Math.max(view.width, view.height)), viewport = page.getViewport({ scale, rotation: 0 });
            const canvas = el("canvas");
            canvas.width = Math.ceil(viewport.width);
            canvas.height = Math.ceil(viewport.height);
            setUiAttr(canvas, "aria-label", displayMessage("media.unrotatedPdfAlt", { p0: uiDisplayParameter(item.name), p1: index + 1 }));
            const renderTask = page.render({ canvas, viewport, annotationMode: 0 });
            rendering = renderTask;
            try {
                await renderTask.promise;
            }
            catch (error) {
                if (!active(ticket) || pageTicket !== pageGeneration)
                    return;
                throw error;
            }
            if (!active(ticket) || pageTicket !== pageGeneration)
                return;
            rendering = null;
            set(restore({ ...base("pdf_region"), pageIndex: index, rotation: facts.rotation, width: facts.width, height: facts.height }));
            drawSurface(canvas, facts.width, facts.height, false);
        };
        pageSelect.onchange = () => {
            void renderPage(Number(pageSelect.value)).catch(error => {
                if (active(ticket)) {
                    set(null);
                    report(message(error), true);
                }
            });
        };
        const startPage = options.open?.pageIndex ?? (remembered?.resource === item.resource && remembered.revision === item.revision && remembered.kind === "pdf_region" ? remembered.pageIndex ?? 0 : 0);
        const pageIndex = Math.max(0, Math.min(loaded.numPages - 1, startPage));
        pageSelect.value = String(pageIndex);
        await renderPage(pageIndex);
    }
    async function showPlayer(item: MediaResource, ticket: number): Promise<void> {
        const metadata = item.metadata;
        if (metadata.kind !== "audio" && metadata.kind !== "video")
            return;
        const native = metadata.kind === "audio" ? el("audio") : el("video");
        player = native;
        native.controls = true;
        native.preload = "metadata";
        native.src = objectUrl!;
        await new Promise<void>((resolve, reject) => {
            const cancel = () => { native.onloadedmetadata = null; native.onerror = null; reject(new UiError(uiMessage("media.viewerRetired"))); };
            cancelDecoding.add(cancel);
            native.onloadedmetadata = () => { cancelDecoding.delete(cancel); resolve(); };
            native.onerror = () => { cancelDecoding.delete(cancel); reject(new UiError(uiMessage("media.decodeFailed"))); };
        });
        if (!active(ticket))
            return;
        const seconds = metadata.durationTick * metadata.timebase.denominator / metadata.timebase.numerator;
        if (!Number.isFinite(native.duration) || Math.abs(native.duration - seconds) > Math.max(metadata.timebase.denominator / metadata.timebase.numerator, 1e-9))
            throw new UiError(uiMessage("media.timelineTimebaseMismatch"));
        if (metadata.kind === "video" && (native instanceof HTMLVideoElement) && (native.videoWidth !== metadata.width || native.videoHeight !== metadata.height))
            throw new UiError(uiMessage("media.videoDimensionsMismatch"));
        if (options.readOnly) {
            preview.append(native);
            setUiText(details, displayMessage("media.readOnlyTimelineSummary", { p0: uiDisplayParameter(metadata.durationTick), p1: uiDisplayParameter(metadata.timebase.denominator), p2: uiDisplayParameter(metadata.timebase.numerator), p3: uiDisplayParameter(metadata.kind === "video" ? displayMessage("media.videoPixelDimensionsSuffix", { p0: uiDisplayParameter(metadata.width), p1: uiDisplayParameter(metadata.height) }) : ""), p4: uiDisplayParameter(item.revision) }));
            return;
        }
        set(restore({ ...base("media_interval"), mediaType: metadata.kind, timebase: metadata.timebase.numerator, timebaseDenominator: metadata.timebase.denominator, durationTick: metadata.durationTick, startTick: 0, endTick: metadata.durationTick, ...(metadata.kind === "video" ? { width: metadata.width, height: metadata.height } : {}) }));
        const timeline = el("div", "media-timeline"), start = el("input", "media-input"), end = el("input", "media-input");
        for (const input of [start, end]) {
            input.type = "number";
            input.min = "0";
            input.max = String(metadata.durationTick);
            input.step = "1";
        }
        start.value = String(current?.startTick ?? 0);
        end.value = String(current?.endTick ?? metadata.durationTick);
        start.oninput = () => {
            if (current)
                set({ ...current, startTick: Number(start.value) });
        };
        end.oninput = () => {
            if (current)
                set({ ...current, endTick: Number(end.value) });
        };
        const capture = (which: "start" | "end"): void => {
            const tick = playerTick(native.currentTime, metadata.timebase.numerator, metadata.timebase.denominator, metadata.durationTick);
            if (tick === null || !current)
                return;
            (which === "start" ? start : end).value = String(tick);
            set({ ...current, [which === "start" ? "startTick" : "endTick"]: tick });
        };
        timeline.append(labeled(uiMessage("media.startTick"), start), button(uiMessage("media.setStartFromPlayer"), () => capture("start")), labeled(uiMessage("media.endTick"), end), button(uiMessage("media.setEndFromPlayer"), () => capture("end")));
        controls.append(timeline, el("p", "media-help", uiMessage("media.playerTickHelp")));
        if (metadata.kind === "video") {
            native.controls = false;
            drawSurface(native as HTMLVideoElement, metadata.width, metadata.height, true);
            // Separate seek controls avoid treating a player-control drag as a region.
            const toggle = button(uiMessage("media.playPause"), () => native.paused ? native.play() : native.pause());
            const seek = el("input", "media-input");
            seek.type = "range";
            seek.min = "0";
            seek.max = String(metadata.durationTick);
            seek.step = "1";
            seek.value = "0";
            setUiAttr(seek, "aria-label", uiMessage("media.currentVideoTickAria"));
            seek.oninput = () => { native.currentTime = Number(seek.value) * metadata.timebase.denominator / metadata.timebase.numerator; };
            native.ontimeupdate = () => { seek.value = String(playerTick(native.currentTime, metadata.timebase.numerator, metadata.timebase.denominator, metadata.durationTick) ?? 0); };
            controls.append(toggle, labeled(uiMessage("media.videoPositionLabel"), seek));
        }
        else
            preview.append(native);
    }
    async function showUnsupported(item: MediaResource, blob: Blob, ticket: number): Promise<void> {
        const text = item.metadata.kind === "unsupported" ? descriptionUiText(item.metadata) : uiMessage("media.exactRegionUnavailable");
        preview.append(el("p", "media-help", text));
        if (item.contentType.startsWith("image/")) {
            const image = el("img", "media-surface");
            image.src = objectUrl!;
            image.alt = item.name;
            preview.append(image);
        }
        else if (item.contentType === "video/mp4" || item.contentType.startsWith("audio/")) {
            const native = item.contentType.startsWith("audio/") ? el("audio") : el("video");
            native.controls = true;
            native.src = objectUrl!;
            player = native;
            preview.append(native);
        }
        else if (item.contentType === "application/pdf") {
            const frame = el("iframe", "media-pdf-fallback");
            frame.src = objectUrl!;
            setUiAttr(frame, "title", item.name);
            frame.setAttribute("sandbox", "");
            preview.append(frame);
        }
        else if (options.readOnly && blob.size <= 2 * 1024 * 1024) {
            try {
                const bytes = new Uint8Array(await blob.arrayBuffer());
                if (!active(ticket))
                    return;
                const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
                if (!/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/u.test(text))
                    preview.append(el("pre", "media-text-view", text));
            }
            catch { /* Binary/invalid UTF-8 remains an exact downloadable resource. */ }
        }
        set(null);
    }
    void refresh();
    return { selection: () => current && validateMediaRegion(current) === null ? { ...current } : null, dispose: () => { disposed = true; generation++; clean(); selected(null); disposeUiTree(container); container.replaceChildren(); } };
}
