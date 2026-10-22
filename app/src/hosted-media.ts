import { message, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, disposeUiTree, type UiText } from "./ui-locale";
import { action, ui } from "./data-panel";
import { mountMediaViewer, type MediaViewer } from "./media-viewer";
import type { MediaRegionInput } from "./knowledge-state";
import type { ResourceOpenTarget } from "./media-selection";
import { HostedResourceImports } from "./hosted-media-import";
import { resourceCollection } from "./hosted-media-import-state";
import { decodeMediaDescriptor, mediaChunk, mediaFingerprint, mediaResolved, mediaTicket, mediaViewerDescriptor, type HostedMediaCatalog, type HostedMediaChunk, type HostedMediaDescriptor, type HostedMediaResolved, type HostedMediaSelection, type HostedMediaTicket, type MediaPurpose } from "./hosted-media-state";
export type HostedMediaRequest = <T>(path: string, body?: unknown, signal?: AbortSignal) => Promise<T>;
/** Exact asset bytes are independent of a note session and never persisted. */
export class HostedMedia {
    private epoch = 0;
    private version = 0;
    private catalogVersion = 0;
    private catalog: HostedMediaCatalog = { resources: [], collections: [], coverage: "complete", scope: "configuredAuthorizedResources" };
    private selected?: HostedMediaDescriptor;
    private target: MediaRegionInput | null = null;
    private viewer?: MediaViewer;
    private controllers = new Set<AbortController>();
    private urls = new Set<string>();
    private picker = ui("select");
    private surface = ui("section");
    private actions = ui("section");
    private message = ui("p", "hint");
    private coverage = ui("p", "hint");
    private importHost = ui("section");
    private imports: HostedResourceImports;
    private downloading = false;
    constructor(private host: HTMLElement, private request: HostedMediaRequest, private selectionChanged: (context: HostedMediaSelection | null) => void, private review: (context: HostedMediaSelection) => void | Promise<void>) { this.imports = new HostedResourceImports(this.importHost, request, () => this.refresh()); this.clear(false); }
    get hasPending() { return this.imports.hasPending; }
    context(): HostedMediaSelection | null {
        if (!this.selected) return null;
        const descriptor = decodeMediaDescriptor(this.selected);
        return descriptor.status === "ready" ? { descriptor, target: this.target && { ...this.target } } : null;
    }
    private stopView() {
        this.version++;
        for (const controller of this.controllers)
            controller.abort();
        this.controllers.clear();
        this.viewer?.dispose();
        this.viewer = undefined;
        for (const url of this.urls)
            URL.revokeObjectURL(url);
        this.urls.clear();
        this.selected = undefined;
        this.target = null;
        this.downloading = false;
        disposeUiTree(this.surface);
        disposeUiTree(this.actions);
        this.surface.replaceChildren();
        this.actions.replaceChildren();
        this.selectionChanged(null);
    }
    clear(removeCache = true) { this.epoch++; this.catalogVersion++; this.stopView(); this.catalog = { resources: [], collections: [], coverage: "complete", scope: "configuredAuthorizedResources" }; this.picker.replaceChildren(); setUiText(this.message, ""); setUiText(this.coverage, ""); this.host.replaceChildren(); this.host.hidden = true; this.imports.clear(removeCache); }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        this.imports.connect(deployment, actor, generation);
        this.host.hidden = false;
        setUiAttr(this.picker, "aria-label", message("hostedMedia.resourcePickerAria"));
        this.picker.onchange = () => { void this.openAsset(this.picker.value).catch(error => this.show(error)); };
        setUiAttr(this.coverage, "aria-label", message("hostedMedia.catalogCoverageAria"));
        this.host.replaceChildren(ui("h2", "", message("hostedMedia.title")), ui("p", "hint", message("hostedMedia.authorizationHelp")), this.picker, action(message("hostedMedia.refreshCatalog"), () => this.refresh().catch(error => this.show(error))), this.coverage, this.message, this.surface, this.actions, this.importHost);
        await this.refresh();
    }
    private show(error: unknown) { setUiText(this.message, error instanceof Error ? errorUiText(error) : message("hostedMedia.resourceUnavailable")); }
    private stamp() { return { epoch: this.epoch, version: this.version }; }
    private current(stamp: ReturnType<HostedMedia["stamp"]>) { return stamp.epoch === this.epoch && stamp.version === this.version; }
    async refresh() {
        const epoch = this.epoch, catalogVersion = ++this.catalogVersion, result = await this.request<HostedMediaCatalog>("media");
        if (epoch !== this.epoch || catalogVersion !== this.catalogVersion)
            return;
        if (!result || result.scope !== "configuredAuthorizedResources" || !["complete", "partial"].includes(result.coverage) || !Array.isArray(result.resources) || !Array.isArray(result.collections) || Object.keys(result).some(key => !["resources", "collections", "coverage", "scope"].includes(key)))
            throw new UiError(message("hostedMedia.catalogScopeMissing"));
        const previous = this.selected;
        this.catalog = { ...result, resources: result.resources.map(decodeMediaDescriptor), collections: result.collections.map(resourceCollection) };
        this.imports.setCatalog(this.catalog.collections ?? []);
        if (previous) {
            const fresh = this.catalog.resources.find(item => item.asset === previous.asset);
            if (!fresh || fresh.status !== "ready" || mediaFingerprint(fresh) !== mediaFingerprint(previous))
                this.stopView();
            else
                this.selected = fresh;
        }
        const empty = ui("option", "", message("hostedMedia.chooseAuthorizedResource"));
        empty.value = "";
        this.picker.replaceChildren(empty, ...this.catalog.resources.map(item => { const option = ui("option", "", () => `${uiTextValue(item.title ?? item.asset)}${uiTextValue(item.lifecycle === "trash" ? message("hostedMedia.trashReadOnlySuffix") : "")}${uiTextValue(item.status === "unavailable" ? message("hostedMedia.unavailableSuffix") : "")}`); option.value = item.asset; option.disabled = item.status === "unavailable" || !item.revision; return option; }));
        this.picker.value = this.selected?.asset ?? "";
        setUiText(this.coverage, result.coverage === "partial" ? message("hostedMedia.catalogPartial") : result.resources.length ? message("hostedMedia.catalogExplicitScopeHelp") : message("hostedMedia.catalogEmptyComplete"));
        this.renderActions();
    }
    async openReference(reference: string, revision?: string, initial?: MediaRegionInput) {
        this.stopView();
        const stamp = this.stamp();
        setUiText(this.message, message("hostedMedia.resolvingStableReference"));
        try {
            const resolved = mediaResolved(await this.request<HostedMediaResolved>("media/resolve", { reference, ...(revision ? { revision } : {}) }), reference, revision);
            if (!this.current(stamp))
                return;
            this.mount(resolved, resolved.viewerTarget, initial);
            setUiText(this.message, message("hostedMedia.stableReferenceResolved"));
        }
        catch (error) {
            if (this.current(stamp)) {
                this.stopView();
                this.show(error);
            }
        }
    }
    async openAsset(asset: string, revision?: string, initial?: MediaRegionInput) {
        const candidate = this.catalog.resources.find(item => item.asset === asset);
        if (!candidate || candidate.status !== "ready" || !candidate.revision) {
            this.stopView();
            setUiText(this.message, message("hostedMedia.selectionUnavailable"));
            return;
        }
        if (revision && revision !== candidate.revision) {
            this.stopView();
            throw new UiError(message("hostedMedia.originalVersionChanged"));
        }
        await this.openReference(candidate.resource, revision ?? candidate.revision, initial);
    }
    private mount(descriptor: HostedMediaDescriptor, open: ResourceOpenTarget, initial?: MediaRegionInput) {
        // The resolved viewer target is presentation state, not part of the
        // catalog/ticket descriptor that context() re-decodes.
        const { viewerTarget: _target, ...selectionDescriptor } = descriptor as HostedMediaResolved;
        this.selected = selectionDescriptor;
        this.picker.value = descriptor.asset;
        const stamp = this.stamp(), viewerId = `hosted-media:${stamp.epoch}:${stamp.version}`;
        this.viewer = mountMediaViewer(this.surface, {
            context: () => this.current(stamp) ? { viewerId } : null,
            list: async () => ({ resources: [mediaViewerDescriptor(descriptor)], warnings: [] }),
            read: async (_lifetime, resource, revision) => {
                if (resource !== descriptor.resource || revision !== descriptor.revision)
                    throw new UiError(message("hostedMedia.viewerRequestMismatch"));
                return this.readBlob(descriptor, "view", stamp);
            },
            notify: (message, error) => {
                if (this.current(stamp)) {
                    if (error)
                        this.stopView();
                    setUiText(this.message, message);
                }
            },
        }, target => {
            if (this.current(stamp)) {
                this.target = target;
                this.selectionChanged(this.context());
            }
        }, initial, { open, fixedResource: true, readOnly: descriptor.lifecycle === "trash" || !descriptor.annotationReady, allowSaveCopy: false });
        this.selectionChanged(this.context());
        this.renderActions();
    }
    private async readBlob(descriptor: HostedMediaDescriptor, purpose: MediaPurpose, stamp = this.stamp()): Promise<Blob> {
        if (purpose === "download" && !descriptor.download)
            throw new UiError(message("hostedMedia.downloadUnauthorized"));
        const controller = new AbortController();
        this.controllers.add(controller);
        try {
            const ticket = mediaTicket(descriptor, purpose, await this.request<HostedMediaTicket>(purpose === "view" ? "media/read" : "media/download/prepare", { asset: descriptor.asset, revision: descriptor.revision }, controller.signal));
            const chunks: Uint8Array<ArrayBuffer>[] = [];
            for (let offset = 0, done = false; !done;) {
                if (!this.current(stamp))
                    throw new UiError(message("hostedMedia.viewContextChanged"));
                const response = await this.request<HostedMediaChunk>(purpose === "view" ? "media/chunk" : "media/download/chunk", { ticket: ticket.ticket, offset }, controller.signal);
                const chunk = mediaChunk(ticket, offset, response);
                chunks.push(new Uint8Array(chunk));
                offset += chunk.length;
                done = response.done;
            }
            const bytes = new Uint8Array(ticket.bytes);
            let offset = 0;
            for (const chunk of chunks) {
                bytes.set(chunk, offset);
                offset += chunk.length;
            }
            const digest = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(n => n.toString(16).padStart(2, "0")).join("");
            if (offset !== ticket.bytes || digest !== ticket.sha256 || !this.current(stamp))
                throw new UiError(message("hostedMedia.blobValidationFailed"));
            return new Blob([bytes], { type: ticket.contentType });
        }
        finally {
            this.controllers.delete(controller);
        }
    }
    private async download() {
        if (!this.selected || !this.selected.download || this.downloading)
            return;
        const descriptor = this.selected, stamp = this.stamp();
        this.downloading = true;
        this.renderActions();
        try {
            const blob = await this.readBlob(descriptor, "download", stamp);
            if (!this.current(stamp))
                return;
            const url = URL.createObjectURL(blob);
            this.urls.add(url);
            const link = ui("a");
            link.href = url;
            link.download = descriptor.name;
            link.click();
            setTimeout(() => { URL.revokeObjectURL(url); this.urls.delete(url); }, 1000);
            setUiText(this.message, message("hostedMedia.downloadComplete"));
        }
        catch (error) {
            if (this.current(stamp))
                this.show(error);
        }
        finally {
            if (this.current(stamp)) {
                this.downloading = false;
                this.renderActions();
            }
        }
    }
    private renderActions() {
        this.actions.replaceChildren();
        const descriptor = this.selected;
        if (!descriptor)
            return;
        this.actions.append(ui("p", "hint", () => `${uiTextValue(descriptor.title)} · ${uiTextValue(descriptor.revision)}${uiTextValue(descriptor.lifecycle === "trash" ? message("hostedMedia.trashAnnotationDisabledSuffix") : "")}`));
        if (descriptor.download) {
            const download = action(message("hostedMedia.downloadExactResource"), () => this.download());
            download.disabled = this.downloading;
            this.actions.append(download);
        }
        const annotations = action(message("hostedMedia.readAnnotationDiscussion"), () => {
            const context = this.context();
            if (context)
                return Promise.resolve(this.review(context)).catch(error => this.show(error));
        });
        this.actions.append(annotations);
    }
}
