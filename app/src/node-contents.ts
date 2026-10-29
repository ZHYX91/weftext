import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { Inventory, InventoryEntry } from "./types";
import { nodeSourceHash } from "./hosted-navigation-state";
import { mountListThumbnail, type ListPreview } from "./list-thumbnail";
import type { MediaResource } from "./media-viewer";
import { contentsActionEntry, contentsPreferenceKey, contentsSection, directContentsEntries, decodeAuthoredOrder, decodeContentsPreference, decodePendingOrder, defaultContentsPreference, insertedSibling, isTerminalOrderStatus, movedSibling, type AuthoredOrder, type ContentsPreference, type ContentsSection, type OrderPlan, type PendingOrder } from "./node-contents-state";
import { canonicalNodeSources } from "./file-tree";
import "./node-contents.css";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
/** The host captures an authorized exact version. A raw inventory path is never a resource URI. */
export interface ContentsResource {
    descriptor: MediaResource;
    read(): Promise<Blob>;
    open(): Promise<void>;
}
export interface ContentsHost {
    request: Request;
    readBlob(path: string, body: unknown): Promise<Blob>;
    open(entry: InventoryEntry): void | Promise<void>;
    browse(path: string | null): void;
    menu(entries: InventoryEntry[], folder?: string): void;
    preview(entries: InventoryEntry[]): void | Promise<void>;
    resource?(entry: InventoryEntry): Promise<ContentsResource | null>;
    changed(): Promise<void>;
    notify(message: UiText): void;
}
const normalized = (path: string) => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
const leaf = (path: string) => path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ?? path;
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = "", className = ""): HTMLElementTagNameMap[K] { const item = document.createElement(tag); setUiText(item, text); item.className = className; return item; }
/** Navigation rows are projections. Only the explicit original Core plan writes
 * shared sibling order; list settings and previews never save author content. */
export function mountNodeContents(panel: HTMLElement, inventory: Inventory, folder: string, query: string, selections: Set<string>, host: ContentsHost): {
    dispose(): void;
} {
    let disposed = false, busy = false, order: AuthoredOrder | undefined, pending: PendingOrder | undefined, parentRef: string | undefined;
    let preference = defaultContentsPreference(), persistenceError: UiText = "", dragging: string | undefined, recoveryBlocked = false;
    const dialogs = new Set<HTMLDialogElement>();
    let thumbnails: {
        dispose(): void;
    }[] = [];
    panel.classList.add("node-resource-view");
    const header = el("header"), rows = el("div", "", "node-content-sections"), controls = el("div", "", "node-contents-controls"), notice = el("p", "", "node-contents-notice"), orderHost = el("div", "", "node-order-controls");
    const direct = directContentsEntries(inventory.entries, folder);
    const canonicalSources = canonicalNodeSources(inventory.entries);
    const visible = direct.filter(entry => !query || `${entry.title ?? ""} ${entry.name} ${entry.relativePath}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
    const button = (label: UiText, run: () => void | Promise<void>) => { const control = el("button", label); control.type = "button"; control.onclick = () => { void Promise.resolve().then(run).catch(error => host.notify(error instanceof Error ? errorUiText(error) : String(error))); }; return control; };
    const key = contentsPreferenceKey(inventory.root, inventory.workspaceId, folder);
    // Pending author operations retain their stable-parent recovery key even
    // when display preferences use a different folder coordinate.
    const pendingKey = () => `weftext-node-contents:${encodeURIComponent(parentRef!)}:operation`;
    try {
        const raw = localStorage.getItem(key);
        if (raw && raw.length <= 8192) preference = decodeContentsPreference(JSON.parse(raw));
    } catch { persistenceError = uiMessage("nodeContents.preferenceRecoveryFailed"); }
    const retainPreference = () => {
        try {
            localStorage.setItem(key, JSON.stringify(preference));
        }
        catch {
            persistenceError = uiMessage("nodeContents.preferencePersistenceFailed");
        }
    };
    const retainPending = () => {
        if (!parentRef)
            throw new UiError(uiMessage("nodeContents.exactParentMissing"));
        if (pending)
            localStorage.setItem(pendingKey(), JSON.stringify(pending));
        else
            localStorage.removeItem(pendingKey());
    };
    const member = (entry: InventoryEntry) => order?.members.find(item => normalized(item.path) === normalized(entry.path));
    const resources = new Map<string, Promise<ContentsResource | null>>();
    const resource = (entry: InventoryEntry): Promise<ContentsResource | null> => {
        let captured = resources.get(entry.path);
        if (!captured) {
            captured = Promise.resolve().then(() => host.resource?.(entry) ?? null);
            resources.set(entry.path, captured);
            // Failed captures are retryable; a successful capture stays pinned
            // to its exact version until this folder projection is refreshed.
            void captured.catch(() => { if (resources.get(entry.path) === captured) resources.delete(entry.path); });
        }
        return captured;
    };
    const openResource = async (entry: InventoryEntry) => {
        const captured = await resource(entry);
        if (disposed) return;
        if (!captured) throw new UiError(uiMessage("nodeContents.resourceUnavailable"));
        await captured.open();
    };
    const stopThumbnails = () => { thumbnails.forEach(item => item.dispose()); thumbnails = []; };
    const runBusy = async (work: () => Promise<void>) => {
        if (busy || disposed)
            return;
        busy = true;
        renderOrder();
        try {
            await work();
        }
        finally {
            busy = false;
            if (!disposed) {
                renderRows();
                renderOrder();
            }
        }
    };
    header.append(button(leaf(inventory.root), () => host.browse(null)));
    const relative = folder.slice(inventory.root.length).replaceAll("\\", "/").split("/").filter(Boolean);
    let ancestor = inventory.root;
    for (const name of relative) {
        ancestor += "/" + name;
        const target = ancestor;
        header.append(el("span", "›"), button(name, () => host.browse(target)));
    }
    const sorting = el("select");
    setUiAttr(sorting, "aria-label", uiMessage("nodeContents.sortLabel"));
    for (const [value, text] of [["name", uiMessage("nodeContents.nameOrder")], ["path", uiMessage("nodeContents.pathOrder")], ["authored", uiMessage("nodeContents.authoredOrder")]] as [
        string,
        UiText
    ][]) {
        const option = el("option", text);
        option.value = value;
        sorting.append(option);
    }
    sorting.onchange = () => { preference.sort = sorting.value as ContentsPreference["sort"]; retainPreference(); renderRows(); renderOrder(); };
    controls.append(sorting);
    for (const [property, label] of [["thumbnails", uiMessage("nodeContents.thumbnailsLabel")], ["summaries", uiMessage("nodeContents.summariesLabel")]] as const) {
        const wrapper = el("label", label), check = el("input");
        check.type = "checkbox";
        check.dataset.preference = property;
        setUiAttr(check, "aria-label", label);
        check.onchange = () => { preference[property] = check.checked; retainPreference(); renderRows(); };
        wrapper.prepend(check);
        controls.append(wrapper);
    }
    const density = el("select");
    setUiAttr(density, "aria-label", uiMessage("nodeContents.densityLabel"));
    for (const [value, label] of [["compact", uiMessage("queryView.compactDensity")], ["comfortable", uiMessage("queryView.detailedDensity")]] as [
        string,
        UiText
    ][]) {
        const option = el("option", label);
        option.value = value;
        density.append(option);
    }
    density.onchange = () => { preference.density = density.value as ContentsPreference["density"]; retainPreference(); renderRows(); };
    controls.append(density);
    controls.append(button(uiMessage("nodeContents.resetPreferences"), () => { preference = defaultContentsPreference(); retainPreference(); renderRows(false); renderOrder(); }));
    const actionEntry = (entry: InventoryEntry) => contentsActionEntry(entry, canonicalSources);
    const selectedAction = button(uiMessage("nodeContents.selectedActions"), () => host.menu(visible.filter(entry => selections.has(entry.path)).map(actionEntry)));
    panel.replaceChildren(header, controls, notice, selectedAction, orderHost, rows);
    function updateSelection() {
        rows.querySelectorAll<HTMLElement>("[data-entry-path]").forEach(row => { const selected = selections.has(row.dataset.entryPath!); row.setAttribute("aria-selected", String(selected)); row.querySelector<HTMLInputElement>("input")!.checked = selected; });
        selectedAction.disabled = !visible.some(entry => selections.has(entry.path));
    }
    function renderRows(preserveScroll = true) {
        if (disposed)
            return;
        stopThumbnails();
        const active = document.activeElement as HTMLElement | null, focusedRow = active?.closest<HTMLElement>("[data-entry-path]"), focusedPath = focusedRow?.dataset.entryPath;
        const focusedRole = focusedRow && panel.contains(focusedRow) ? active?.dataset.controlRole : undefined;
        const focusedSectionControl = active && panel.contains(active) ? active.dataset.sectionControl : undefined;
        const scroll = panel.scrollTop;
        if (preserveScroll) for (const body of rows.querySelectorAll<HTMLElement>("[data-section-body]"))
            preference.sections[body.dataset.sectionBody as ContentsSection].scroll = body.scrollTop;
        rows.replaceChildren();
        sorting.value = preference.sort;
        density.value = preference.density;
        controls.querySelectorAll<HTMLInputElement>("input[type=checkbox]").forEach(check => { check.checked = preference[check.dataset.preference as "thumbnails" | "summaries"]; });
        panel.dataset.density = preference.density;
        const rank = new Map(order?.orderedRefs.map((ref, index) => [ref, index]) ?? []);
        const entries = [...visible].sort((a, b) => {
            if (preference.sort === "authored") {
                const difference = (rank.get(member(a)?.targetRef ?? "") ?? Number.MAX_SAFE_INTEGER) - (rank.get(member(b)?.targetRef ?? "") ?? Number.MAX_SAFE_INTEGER);
                if (difference)
                    return difference;
            }
            return (preference.sort === "path" ? a.relativePath : a.title || a.name).localeCompare(preference.sort === "path" ? b.relativePath : b.title || b.name, "zh-CN");
        });
        const sectionBodies = new Map<ContentsSection, HTMLElement>();
        for (const [id, label] of [["nodes", uiMessage("nodeContents.childNodes")], ["album", uiMessage("nodeContents.album")], ["files", uiMessage("nodeContents.files")]] as const) {
            const group = el("section", "", `node-contents-section node-contents-${id}`), body = el("div", "", "node-contents-section-body");
            group.dataset.contentsSection = id;
            body.id = `node-contents-${id}-${encodeURIComponent(folder)}`;
            body.dataset.sectionBody = id;
            const count = entries.filter(entry => contentsSection(entry) === id).length;
            const toggle = button(label, () => { preference.sections[id].collapsed = !preference.sections[id].collapsed; retainPreference(); renderRows(); });
            toggle.className = "node-contents-section-heading";
            toggle.dataset.sectionControl = `${id}-toggle`;
            toggle.setAttribute("aria-expanded", String(!preference.sections[id].collapsed));
            toggle.setAttribute("aria-controls", body.id);
            toggle.append(el("span", String(count), "node-contents-count"));
            body.hidden = preference.sections[id].collapsed;
            body.onscroll = () => { preference.sections[id].scroll = body.scrollTop; retainPreference(); };
            group.append(toggle, body);
            rows.append(group);
            sectionBodies.set(id, body);
        }
        const sectionSeen: Record<ContentsSection, number> = { nodes: 0, album: 0, files: 0 };
        for (const entry of entries) {
            const section = contentsSection(entry);
            if (++sectionSeen[section] > preference.sections[section].limit) continue;
            const row = el("div", "", "node-content-row");
            row.tabIndex = 0;
            row.dataset.entryPath = entry.path;
            const check = el("input");
            check.dataset.controlRole = "selection";
            check.type = "checkbox";
            setUiAttr(check, "aria-label", displayMessage("nodeContents.selectEntryLabel", { p0: uiDisplayParameter(entry.name) }));
            check.onchange = () => { check.checked ? selections.add(entry.path) : selections.delete(entry.path); updateSelection(); };
            const actionTarget = actionEntry(entry), paired = actionTarget !== entry;
            const open = () => {
                if (entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ")) {
                    return openResource(entry);
                }
                return host.open(actionTarget);
            }, identityLabel = entry.identityStatus === "bindingConflict" ? uiMessage("shell.fileStatusInvalidIdentity") : entry.identityStatus,
                title = button(() => `${uiTextValue(entry.title || entry.name)} · ${uiTextValue(paired || entry.identityStatus === "admitted" ? uiMessage("queryColumn.note") : entry.kind === "directory" ? uiMessage("nodeContents.directoryKind") : identityLabel)}`, () => { selections.clear(); selections.add(entry.path); updateSelection(); });
            title.dataset.controlRole = "title";
            if (paired) {
                // A healthy node opens its note in one click. The second click
                // of a native double-click must not dispatch a second open.
                title.onclick = event => {
                    if (event.detail > 1) return;
                    selections.clear(); selections.add(entry.path); updateSelection();
                    void Promise.resolve().then(open).catch(error => host.notify(error instanceof Error ? errorUiText(error) : String(error)));
                };
            } else title.ondblclick = () => { void Promise.resolve(open()).catch(error => host.notify(String(error))); };
            const menu = () => {
                if (!selections.has(entry.path)) {
                    selections.clear();
                    selections.add(entry.path);
                    updateSelection();
                }
                host.menu(visible.filter(item => selections.has(item.path)).map(actionEntry), entry.kind === "directory" ? entry.path : undefined);
            };
            row.oncontextmenu = event => { event.preventDefault(); menu(); };
            row.onkeydown = event => {
                if (event.isComposing)
                    return;
                if (event.key === "Enter" && event.target === row) {
                    event.preventDefault();
                    void Promise.resolve(open()).catch(error => host.notify(String(error)));
                }
                if (event.key === "ContextMenu" || event.shiftKey && event.key === "F10") {
                    event.preventDefault();
                    menu();
                }
            };
            const openButton = button(paired ? uiMessage("nodeContents.openNote") : uiMessage("common.open"), open);
            const ordinary = entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ");
            openButton.disabled = entry.kind === "blocked" || ordinary && !host.resource;
            if (openButton.disabled && ordinary) setUiAttr(openButton, "title", uiMessage("nodeContents.resourceUnavailable"));
            const previewButton = button(paired ? uiMessage("nodeContents.browseContents") : uiMessage("nodeContents.preview"), () => paired ? host.browse(entry.path) : ordinary && host.resource ? openResource(entry) : host.preview([entry]));
            openButton.dataset.controlRole = "open";
            previewButton.dataset.controlRole = "preview";
            row.append(check, title, openButton, previewButton);
            if (entry.identityStatus === "bindingConflict") {
                const warning = el("span", identityLabel, "identity-badge");
                setUiAttr(warning, "title", entry.identityStatus);
                row.append(warning);
            }
            const source = entry.kind === "directory" ? canonicalSources.get(normalized(entry.path)) : entry;
            if (preference.summaries)
                row.append(el("p", ordinary ? uiMessage("nodeContents.readOnlyResource") : source?.excerpt ?? uiMessage("nodeContents.summaryUnavailable"), "node-content-summary"));
            if (preference.thumbnails) {
                const thumbnail = el("div", "", "node-content-thumbnail");
                row.append(thumbnail);
                if (ordinary && section === "album" && host.resource && entry.kind !== "blocked") {
                    let cancelled = false, started = false, renderer: { dispose(): void } | undefined;
                    setUiText(thumbnail, uiMessage("thumbnail.waitingVisible"));
                    const start = () => {
                        if (cancelled || started) return;
                        started = true;
                        setUiText(thumbnail, uiMessage("thumbnail.building"));
                        void resource(entry).then(async captured => {
                            if (cancelled || disposed) return;
                            if (!captured || !["image", "pdf"].includes(captured.descriptor.metadata.kind)) {
                                setUiText(thumbnail, uiMessage("thumbnail.unsupported"));
                                return;
                            }
                            const media = await import("./media-viewer");
                            if (cancelled || disposed) return;
                            renderer = media.renderQueryThumbnail(thumbnail, captured.descriptor, captured.read);
                        }).catch(error => { if (!cancelled && !disposed) setUiText(thumbnail, errorUiText(error)); });
                    };
                    const observer = typeof IntersectionObserver === "undefined" ? undefined : new IntersectionObserver(items => {
                        if (items.some(item => item.isIntersecting)) { observer!.disconnect(); start(); }
                    }, { root: panel, rootMargin: "120px" });
                    if (observer) observer.observe(thumbnail); else queueMicrotask(start);
                    thumbnails.push({ dispose() { cancelled = true; observer?.disconnect(); renderer?.dispose(); } });
                }
                else if (source?.nodeId && source.identityStatus === "admitted" && source.source !== undefined && inventory.workspaceId) {
                    const entityKey = `weftext-node://${inventory.workspaceId}/${source.nodeId}`, rowSource = source.source;
                    thumbnails.push(mountListThumbnail(thumbnail, { viewport: panel,
                        resolve: async () => host.request<ListPreview>("/api/navigation/preview", { root: inventory.root, entityKey, sourceHash: await nodeSourceHash(rowSource) }),
                        read: (descriptor, revision) => host.readBlob("/api/query/preview-resource", { root: inventory.root, entityKey, revision, resource: descriptor.resource, resourceRevision: descriptor.revision }) }));
                }
                else {
                    thumbnail.dataset.previewState = source?.identityStatus === "admitted" ? "unavailable" : "unsupported";
                    setUiText(thumbnail, source?.identityStatus === "admitted" ? uiMessage("nodeContents.thumbnailBasisMissing") : uiMessage("nodeContents.thumbnailAuthorityMissing"));
                }
            }
            const target = member(entry)?.targetRef;
            if (preference.sort === "authored" && order?.complete && target && !pending && !recoveryBlocked) {
                const actions = el("div", "", "node-sibling-actions");
                for (const [offset, label] of [[-1, uiMessage("queryShortcut.moveUp")], [1, uiMessage("nodeContents.moveDown")]] as const) {
                    const move = button(label, () => propose(movedSibling(order!.orderedRefs, target, offset)));
                    move.dataset.controlRole = offset < 0 ? "move-up" : "move-down";
                    setUiAttr(move, "aria-label", displayMessage("nodeContents.moveLabel", { p0: uiDisplayParameter(entry.name), p1: uiDisplayParameter(label) }));
                    const index = order.orderedRefs.indexOf(target);
                    move.disabled = busy || offset === -1 && index === 0 || offset === 1 && index === order.orderedRefs.length - 1;
                    actions.append(move);
                }
                row.append(actions);
                row.draggable = true;
                row.ondragstart = event => { dragging = target; event.dataTransfer?.setData("text/plain", target); };
                row.ondragover = event => {
                    if (dragging && dragging !== target) {
                        event.preventDefault();
                        row.dataset.insert = event.clientY > row.getBoundingClientRect().top + row.offsetHeight / 2 ? "after" : "before";
                    }
                };
                row.ondragleave = () => { delete row.dataset.insert; };
                row.ondragend = () => { dragging = undefined; rows.querySelectorAll<HTMLElement>("[data-insert]").forEach(item => { delete item.dataset.insert; }); };
                row.ondrop = event => {
                    event.preventDefault();
                    const sourceRef = dragging, after = row.dataset.insert === "after";
                    delete row.dataset.insert;
                    dragging = undefined;
                    if (sourceRef)
                        propose(insertedSibling(order!.orderedRefs, sourceRef, target, after));
                };
            }
            sectionBodies.get(section)!.append(row);
        }
        for (const [id, body] of sectionBodies) {
            if (!sectionSeen[id]) body.append(el("p", query ? uiMessage("nodeContents.noFilterMatch") : uiMessage("nodeContents.emptySection"), "node-contents-empty"));
            if (sectionSeen[id] > preference.sections[id].limit) {
                const more = button(uiMessage("nodeContents.showMore"), () => { preference.sections[id].limit = Math.min(5000, preference.sections[id].limit + 24); retainPreference(); renderRows(); });
                more.dataset.sectionControl = `${id}-more`;
                body.append(more);
            }
            body.scrollTop = preference.sections[id].scroll;
        }
        updateSelection();
        panel.scrollTop = scroll;
        if (focusedPath && focusedRole !== undefined) {
            const replacement = [...rows.querySelectorAll<HTMLElement>("[data-entry-path]")].find(item => item.dataset.entryPath === focusedPath);
            const control = [...replacement?.querySelectorAll<HTMLElement>("button,input") ?? []].find(item => item.dataset.controlRole === focusedRole);
            control?.focus({ preventScroll: true });
        }
        else if (focusedSectionControl) {
            [...rows.querySelectorAll<HTMLElement>("[data-section-control]")].find(item => item.dataset.sectionControl === focusedSectionControl)?.focus({ preventScroll: true });
        }
    }
    function renderOrder() {
        if (disposed)
            return;
        orderHost.replaceChildren();
        setUiText(notice, () => `${uiTextValue(preference.sort === "authored" ? uiMessage("nodeContents.authoredOrderHelp") : uiMessage("nodeContents.personalOrderHelp"))}${uiTextValue(persistenceError)}`);
        if (!parentRef) {
            if (preference.sort === "authored")
                orderHost.append(el("p", uiMessage("nodeContents.parentRegistrationRequired")));
            return;
        }
        if (order && !order.complete)
            orderHost.append(el("p", uiMessage("nodeContents.orderScopeIncomplete")));
        if (pending) {
            const originalPending = pending;
            orderHost.append(el("p", displayMessage("nodeContents.originalOrderStatus", { p0: uiDisplayParameter(originalPending.request.operationId), p1: uiDisplayParameter(originalPending.status) })));
            const inspect = button(uiMessage("nodeContents.inspectOriginalOrder"), () => runBusy(inspectOriginalOrder));
            inspect.disabled = busy;
            orderHost.append(inspect);
            if (!pending.plan && !pending.attempted) {
                const retry = button(uiMessage("nodeContents.retryOriginalPrepare"), () => runBusy(prepare));
                retry.disabled = busy;
                orderHost.append(retry);
            }
            if (pending.plan && pending.status === "prepared") {
                const confirm = button(uiMessage("nodeContents.confirmOriginalChanges"), () => showProposal(pending!.request.children, true));
                confirm.disabled = busy;
                orderHost.append(confirm);
                const cancel = button(uiMessage("nodeContents.cancelOriginalPlan"), () => runBusy(async () => { await receive(await host.request<OrderPlan>("/api/navigation/order/cancel", { root: inventory.root, operationId: pending!.request.operationId, planId: pending!.plan!.planId })); }));
                cancel.disabled = busy;
                orderHost.append(cancel);
            }
        }
    }
    async function inspectOriginalOrder(): Promise<void> {
        if (!pending) return;
        await receive(await host.request<OrderPlan>("/api/operations/status", { operationId: pending.request.operationId }));
    }
    async function receive(plan: OrderPlan): Promise<void> {
        // The old owner may finish a request after navigation remounts this
        // parent. It must never modify the replacement owner's recovery key.
        if (disposed) return;
        if (!pending || plan.operationId !== pending.request.operationId || pending.plan && plan.planId && pending.plan.planId !== plan.planId)
            throw new UiError(uiMessage("nodeContents.orderReceiptMismatch"));
        const original = pending;
        if (plan.planId)
            original.plan = plan;
        original.status = plan.status;
        retainPending();
        if (plan.status === "committed") {
            host.notify(uiMessage("nodeContents.orderCommitted"));
            // The host updates the library/tree without retiring this owner.
            // A failed library refresh retains the known original outcome.
            await host.changed();
        }
        if (!isTerminalOrderStatus(plan.status) || disposed || pending !== original || original.status !== plan.status)
            return;
        const fresh = await refreshOrder();
        if (disposed || pending !== original || original.status !== plan.status || recoveryBlocked)
            return;
        if (!fresh?.complete)
            throw new UiError(uiMessage("nodeContents.orderScopeIncomplete"));
        // Remove the durable record first. A storage error keeps the original
        // in memory as well, so it cannot silently enable a new operation.
        localStorage.removeItem(pendingKey());
        pending = undefined;
    }
    async function prepare() {
        const selected = pending!;
        await receive(await host.request<OrderPlan>("/api/navigation/order/prepare", structuredClone(selected.request)));
    }
    function propose(refs: string[]) {
        if (busy || pending || recoveryBlocked || !order?.complete || refs.join("\n") === order.orderedRefs.join("\n"))
            return;
        showProposal(refs, false);
    }
    function showProposal(refs: string[], prepared: boolean) {
        const selectedOrder = order, original = pending;
        if (!selectedOrder)
            return;
        const trigger = document.activeElement as HTMLElement | null, dialog = el("dialog", "", "everyday-dialog");
        setUiAttr(dialog, "aria-label", uiMessage("nodeContents.orderDialogTitle"));
        const originalOrder = prepared ? pending!.beforeRefs : selectedOrder.orderedRefs, names = (items: string[]) => joinUiText(items.map(ref => () => `${uiTextValue(selectedOrder.members.find(item => item.targetRef === ref)?.name ?? uiMessage("nodeContents.unavailableStableObject"))}\n${ref}`), "\n\n");
        dialog.append(el("h2", uiMessage("nodeContents.orderDialogTitle")), el("p", displayMessage("nodeContents.parentSummary", { p0: uiDisplayParameter(selectedOrder.parentRef) })), el("p", uiMessage("nodeContents.orderWriteScopeHelp")), el("h3", uiMessage("nodeContents.originalOrderHeading")), el("pre", names(originalOrder)), el("h3", uiMessage("nodeContents.proposedOrderHeading")), el("pre", names(refs)));
        if (prepared)
            dialog.append(el("pre", pending?.plan?.changes ? joinUiText(pending.plan.changes.map(change => displayMessage("nodeContents.planChangeSummary", { p0: uiDisplayParameter(change.action), p1: uiDisplayParameter(change.path), p2: change.bytes })), "\n") : uiMessage("nodeContents.checkOriginalPlan")));
        const close = () => {
            dialog.close();
            dialog.remove();
            dialogs.delete(dialog);
            if (trigger?.isConnected)
                trigger.focus({ preventScroll: true });
        };
        dialog.append(button(prepared ? uiMessage("nodeContents.retainWithoutCommit") : uiMessage("common.cancel"), close));
        const apply = button(prepared ? uiMessage("nodeContents.commitOriginalOrder") : uiMessage("nodeContents.prepareOrder"), async () => {
            apply.disabled = true;
            try {
                if (prepared) {
                    if (pending !== original || pending?.status !== "prepared" || !pending.plan)
                        throw new UiError(uiMessage("nodeContents.originalChanged"));
                    await runBusy(async () => { pending!.attempted = true; pending!.status = "unknown"; retainPending(); await receive(await host.request<OrderPlan>("/api/operations/commit", { operationId: pending!.request.operationId, planId: pending!.plan!.planId })); });
                }
                else {
                    if (pending || order !== selectedOrder || recoveryBlocked)
                        throw new UiError(uiMessage("nodeContents.parentBasisChanged"));
                    pending = { request: { root: inventory.root, parentRef: selectedOrder.parentRef, children: refs, baseId: selectedOrder.baseId, operationId: crypto.randomUUID() }, beforeRefs: [...selectedOrder.orderedRefs], status: "preparing" };
                    try {
                        retainPending();
                    }
                    catch (error) {
                        pending = undefined;
                        throw error;
                    }
                    await runBusy(prepare);
                }
                close();
                renderOrder();
                renderRows();
            }
            catch (error) {
                apply.disabled = false;
                host.notify(error instanceof Error ? errorUiText(error) : String(error));
                renderOrder();
            }
        });
        dialog.append(apply);
        dialog.oncancel = event => { event.preventDefault(); close(); };
        document.body.append(dialog);
        dialogs.add(dialog);
        dialog.showModal();
    }
    async function refreshOrder(): Promise<AuthoredOrder | undefined> {
        if (!parentRef)
            return;
        const read = decodeAuthoredOrder(await host.request<AuthoredOrder>("/api/navigation/order/read", { root: inventory.root, parentRef }), parentRef);
        if (disposed)
            return;
        order = read;
        renderRows();
        renderOrder();
        return read;
    }
    renderRows();
    renderOrder();
    void (async () => {
        try {
            const target = await host.request<{
                targetRef?: string;
                status: string;
            }>("/api/navigation/capture", { root: inventory.root, path: folder });
            if (disposed)
                return;
            parentRef = target.targetRef;
            if (!parentRef || target.status !== "managed") {
                parentRef = undefined;
                renderOrder();
                return;
            }
            try {
                const original = localStorage.getItem(pendingKey());
                if (original)
                    pending = decodePendingOrder(original, inventory.root, parentRef);
            }
            catch (error) {
                recoveryBlocked = true;
                persistenceError = displayMessage("nodeContents.recoveryBlocked", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) });
            }
            if (pending && isTerminalOrderStatus(pending.status))
                await runBusy(inspectOriginalOrder);
            else
                await refreshOrder();
        }
        catch (error) {
            if (!disposed) {
                persistenceError = error instanceof Error ? errorUiText(error) : String(error);
                renderOrder();
            }
        }
    })();
    return { dispose() {
            for (const body of rows.querySelectorAll<HTMLElement>("[data-section-body]"))
                preference.sections[body.dataset.sectionBody as ContentsSection].scroll = body.scrollTop;
            retainPreference();
            disposed = true;
            stopThumbnails();
            for (const dialog of dialogs) {
                dialog.close();
                dialog.remove();
            }
            dialogs.clear();
        } };
}
