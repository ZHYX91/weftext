import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { Inventory, InventoryEntry } from "./types";
import { nodeSourceHash } from "./hosted-navigation-state";
import { mountListThumbnail, type ListPreview } from "./list-thumbnail";
import { decodeContentsPreference, decodePendingOrder, defaultContentsPreference, insertedSibling, movedSibling, type AuthoredOrder, type ContentsPreference, type OrderPlan, type PendingOrder } from "./node-contents-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
interface ContentsHost {
    request: Request;
    readBlob(path: string, body: unknown): Promise<Blob>;
    open(entry: InventoryEntry): void | Promise<void>;
    browse(path: string | null): void;
    menu(entries: InventoryEntry[], folder?: string): void;
    preview(entries: InventoryEntry[]): void | Promise<void>;
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
    let preference = defaultContentsPreference(), persistenceError: UiText = "", dragging: string | undefined, preferenceEdited = false, recoveryBlocked = false;
    const dialogs = new Set<HTMLDialogElement>();
    let thumbnails: {
        dispose(): void;
    }[] = [];
    const header = el("header"), rows = el("div"), controls = el("div", "", "node-contents-controls"), notice = el("p"), orderHost = el("div", "", "node-order-controls");
    const direct = inventory.entries.filter(entry => normalized(entry.parentPath) === normalized(folder) && normalized(entry.path) !== normalized(folder));
    const visible = direct.filter(entry => !query || `${entry.title ?? ""} ${entry.name} ${entry.relativePath}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
    const button = (label: UiText, run: () => void | Promise<void>) => { const control = el("button", label); control.type = "button"; control.onclick = () => { void Promise.resolve().then(run).catch(error => host.notify(error instanceof Error ? errorUiText(error) : String(error))); }; return control; };
    const key = () => `weftext-node-contents:${encodeURIComponent(parentRef!)}`;
    const pendingKey = () => `${key()}:operation`;
    const retainPreference = () => {
        preferenceEdited = true;
        if (!parentRef)
            return;
        try {
            localStorage.setItem(key(), JSON.stringify(preference));
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
            if (!disposed)
                renderOrder();
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
    controls.append(button(uiMessage("nodeContents.resetPreferences"), () => { preference = defaultContentsPreference(); retainPreference(); renderRows(); renderOrder(); }));
    const selectedAction = button(uiMessage("nodeContents.selectedActions"), () => host.menu(visible.filter(entry => selections.has(entry.path))));
    panel.replaceChildren(header, controls, notice, selectedAction, orderHost, rows);
    function updateSelection() {
        rows.querySelectorAll<HTMLElement>("[data-entry-path]").forEach(row => { const selected = selections.has(row.dataset.entryPath!); row.setAttribute("aria-selected", String(selected)); row.querySelector<HTMLInputElement>("input")!.checked = selected; });
        selectedAction.disabled = !visible.some(entry => selections.has(entry.path));
    }
    function renderRows() {
        if (disposed)
            return;
        stopThumbnails();
        const active = document.activeElement as HTMLElement | null, focusedRow = active?.closest<HTMLElement>("[data-entry-path]"), focusedPath = focusedRow?.dataset.entryPath;
        const focusedRole = focusedRow && panel.contains(focusedRow) ? active?.dataset.controlRole : undefined;
        const scroll = panel.scrollTop;
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
        for (const entry of entries) {
            const row = el("div", "", "node-content-row");
            row.tabIndex = 0;
            row.dataset.entryPath = entry.path;
            const check = el("input");
            check.dataset.controlRole = "selection";
            check.type = "checkbox";
            setUiAttr(check, "aria-label", displayMessage("nodeContents.selectEntryLabel", { p0: uiDisplayParameter(entry.name) }));
            check.onchange = () => { check.checked ? selections.add(entry.path) : selections.delete(entry.path); updateSelection(); };
            const open = () => {
                if (entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ")) {
                    host.notify(uiMessage("nodeContents.bodyEntryUnsupported"));
                    return;
                }
                return host.open(entry);
            }, title = button(() => `${uiTextValue(entry.title || entry.name)} · ${uiTextValue(entry.identityStatus === "admitted" ? uiMessage("queryColumn.note") : entry.kind === "directory" ? uiMessage("nodeContents.directoryKind") : entry.identityStatus)}`, () => { selections.clear(); selections.add(entry.path); updateSelection(); });
            title.dataset.controlRole = "title";
            title.ondblclick = () => { void Promise.resolve(open()).catch(error => host.notify(String(error))); };
            const menu = () => {
                if (!selections.has(entry.path)) {
                    selections.clear();
                    selections.add(entry.path);
                    updateSelection();
                }
                host.menu(visible.filter(item => selections.has(item.path)), entry.kind === "directory" ? entry.path : undefined);
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
            const openButton = button(uiMessage("common.open"), open);
            openButton.disabled = entry.kind !== "directory" && !entry.path.toLowerCase().endsWith(".typ");
            const previewButton = button(uiMessage("nodeContents.preview"), () => host.preview([entry]));
            openButton.dataset.controlRole = "open";
            previewButton.dataset.controlRole = "preview";
            row.append(check, title, openButton, previewButton);
            const source = entry.kind === "directory" ? inventory.entries.find(item => item.kind === "node" && normalized(item.parentPath) === normalized(entry.path)) : entry;
            if (preference.summaries)
                row.append(el("p", source?.excerpt ?? uiMessage("nodeContents.summaryUnavailable"), "node-content-summary"));
            if (preference.thumbnails) {
                const thumbnail = el("div", "", "node-content-thumbnail");
                row.append(thumbnail);
                if (source?.nodeId && source.identityStatus === "admitted" && source.source !== undefined && inventory.workspaceId) {
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
            rows.append(row);
        }
        if (!entries.length)
            rows.append(el("p", query ? uiMessage("nodeContents.noFilterMatch") : uiMessage("nodeContents.emptyNode")));
        updateSelection();
        panel.scrollTop = scroll;
        if (focusedPath && focusedRole !== undefined) {
            const replacement = [...rows.querySelectorAll<HTMLElement>("[data-entry-path]")].find(item => item.dataset.entryPath === focusedPath);
            const control = [...replacement?.querySelectorAll<HTMLElement>("button,input") ?? []].find(item => item.dataset.controlRole === focusedRole);
            control?.focus({ preventScroll: true });
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
            const inspect = button(uiMessage("nodeContents.inspectOriginalOrder"), () => runBusy(async () => { receive(await host.request<OrderPlan>("/api/operations/status", { operationId: pending!.request.operationId })); }));
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
                const cancel = button(uiMessage("nodeContents.cancelOriginalPlan"), () => runBusy(async () => { receive(await host.request<OrderPlan>("/api/navigation/order/cancel", { root: inventory.root, operationId: pending!.request.operationId, planId: pending!.plan!.planId })); }));
                cancel.disabled = busy;
                orderHost.append(cancel);
            }
            if (["committed", "resolved", "cancelled"].includes(pending.status))
                orderHost.append(button(uiMessage("nodeContents.finishOriginalOperation"), () => { pending = undefined; retainPending(); renderOrder(); renderRows(); }));
        }
    }
    function receive(plan: OrderPlan) {
        if (!pending || plan.operationId !== pending.request.operationId || pending.plan && plan.planId && pending.plan.planId !== plan.planId)
            throw new UiError(uiMessage("nodeContents.orderReceiptMismatch"));
        if (plan.planId)
            pending.plan = plan;
        pending.status = plan.status;
        retainPending();
        if (plan.status === "committed") {
            host.notify(uiMessage("nodeContents.orderCommitted"));
            void host.changed().catch(error => host.notify(String(error)));
        }
    }
    async function prepare() {
        const selected = pending!;
        receive(await host.request<OrderPlan>("/api/navigation/order/prepare", structuredClone(selected.request)));
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
                    await runBusy(async () => { pending!.attempted = true; pending!.status = "unknown"; retainPending(); receive(await host.request<OrderPlan>("/api/operations/commit", { operationId: pending!.request.operationId, planId: pending!.plan!.planId })); });
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
    async function refreshOrder() {
        if (!parentRef)
            return;
        const read = await host.request<AuthoredOrder>("/api/navigation/order/read", { root: inventory.root, parentRef });
        if (disposed)
            return;
        if (read.parentRef !== parentRef || !read.baseId || !Array.isArray(read.members) || !Array.isArray(read.orderedRefs))
            throw new UiError(uiMessage("nodeContents.parentReceiptMismatch"));
        order = read;
        renderRows();
        renderOrder();
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
                const raw = localStorage.getItem(key());
                if (!preferenceEdited && raw && raw.length <= 4096)
                    preference = decodeContentsPreference(JSON.parse(raw));
                else if (preferenceEdited)
                    retainPreference();
            }
            catch {
                persistenceError = uiMessage("nodeContents.preferenceRecoveryFailed");
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
            disposed = true;
            stopThumbnails();
            for (const dialog of dialogs) {
                dialog.close();
                dialog.remove();
            }
            dialogs.clear();
        } };
}
