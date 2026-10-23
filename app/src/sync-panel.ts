import { disposeUiTree, joinUiText, replaceUiChildren, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { api, operationId } from "./api";
import { SyncFlow, decodeReceivedSyncVersions, decodeSyncBinding, decodeSyncIdentity, decodeSyncPending, decodeSyncRevalidationCut, decodeSyncStatus, freezeSyncAdoptionRequest, objectKey, sameSyncIdentity, type ObjectKey, type SyncAdmission, type SyncAdoptionCut, type SyncBinding, type SyncIdentity, type SyncObject, type SyncPending, type SyncStatus, type SyncVersion } from "./sync-state";
import { coreDescriptionUiText } from "./ui-diagnostics";
import { knowledgeLossMessage } from "./knowledge-state";
import { apiErrorUiText } from "./api-error";
import "./sync-panel.css";
import { mountRemoteSync } from "./sync-remote";
import { appendSyncRevalidationSummary, mountSyncRevalidation } from "./sync-revalidation-panel";
interface SyncHost {
    beforeAction(): Promise<void>;
    applied(root: string): Promise<void>;
}
const savedKey = "weftext-sync-operation-v1";
function admissionFacts(admission: SyncAdmission): unknown { return admission.status === "rejected" ? { status: admission.status } : admission; }
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] { const e = document.createElement(tag); setUiText(e, text); return e; }
function field(label: UiText, value = ""): {
    label: HTMLLabelElement;
    input: HTMLInputElement;
} {
    const wrapper = el("label", label), input = el("input");
    input.value = value;
    wrapper.append(input);
    return { label: wrapper, input };
}
export async function openSyncPanel(root: string, host: SyncHost): Promise<void> {
    const saved = localStorage.getItem(savedKey);
    const restored = saved ? decodeSyncPending(JSON.parse(saved)) : null;
    const flow = new SyncFlow({ request: api, store: p => {
            if (p)
                localStorage.setItem(savedKey, JSON.stringify(p));
            else
                localStorage.removeItem(savedKey);
        } }, restored);
    const dialog = el("dialog");
    dialog.className = "sync-panel";
    setUiAttr(dialog, "aria-label", uiMessage("syncPanel.title"));
    const rootField = field(uiMessage("syncPanel.rootLabel"), restored?.request.root ?? root);
    const pairLabel = el("label", uiMessage("syncPanel.pairIdentityLabel")), pair = el("textarea");
    pair.rows = 3;
    pairLabel.append(pair);
    if (restored)
        pair.value = JSON.stringify(restored.request.intent.identity);
    const transport = field(uiMessage("syncPanel.transportLabel"));
    const original = field(uiMessage("syncPanel.originalOperationLabel"));
    const statusLine = el("p");
    statusLine.setAttribute("role", "status");
    const info = el("p"), objectsHost = el("section"), pendingHost = el("section"), controls = el("div"), historyHost = el("section"), inspectHost = el("section"), revalidationHost = el("section");
    setUiAttr(historyHost, "aria-label", uiMessage("syncPanel.receivedHistoryLabel"));
    let binding: SyncBinding | null = null, status: SyncStatus | null = null, busy = false;
    const selected = new Set<string>();
    let selectedInitialized = false;
    let remoteControls: ReturnType<typeof mountRemoteSync> | undefined;
    let revalidation: ReturnType<typeof mountSyncRevalidation> | undefined, reviewGeneration = 0;
    function clearReview(): void { reviewGeneration++; revalidation?.dispose(); revalidation = undefined; replaceUiChildren(revalidationHost); }
    const identity = (): SyncIdentity => {
        return decodeSyncIdentity(JSON.parse(pair.value));
    };
    const targetRoot = (): string => {
        if (!rootField.input.value.trim())
            throw new UiError(uiMessage("syncValidation.localRootRequired"));
        return rootField.input.value;
    };
    const run = async (action: () => void | Promise<void>): Promise<void> => {
        if (busy)
            return;
        busy = true;
        lock();
        setUiText(statusLine, uiMessage("syncPanel.working"));
        try {
            await action();
            setUiText(statusLine, uiMessage("syncPanel.actionComplete"));
        }
        catch (error) {
            setUiText(statusLine, joinUiText([errorUiText(error), flow.pending ? uiMessage("syncPanel.originalRetainedSuffix") : ""]));
        }
        finally {
            busy = false;
            render();
        }
    };
    function button(label: UiText, action: () => void | Promise<void>): HTMLButtonElement {
        const b = el("button", label);
        b.type = "button";
        b.onclick = () => { void run(action); };
        return b;
    }
    function lock(): void { dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = true; }); }
    async function load(): Promise<void> {
        clearReview();
        const incoming = await api<unknown>("/api/sync/binding", { root: targetRoot() });
        binding = incoming === null ? null : decodeSyncBinding(incoming);
        if (binding)
            pair.value = JSON.stringify(binding.identity);
        status = binding ? decodeSyncStatus(await api<unknown>("/api/sync/status", { identity: binding.identity })) : null;
        selectedInitialized = false;
        replaceUiChildren(historyHost);
        replaceUiChildren(inspectHost);
    }
    async function published(): Promise<void> {
        clearReview();
        status = decodeSyncStatus(await api<unknown>("/api/sync/publish", { root: targetRoot() }));
        binding = decodeSyncBinding(await api<unknown>("/api/sync/binding", { root: targetRoot() }));
    }
    async function refreshCommitted(): Promise<void> {
        if (flow.pending?.status === "committed") {
            await host.applied(flow.pending.request.root);
            await load();
            replaceUiChildren(inspectHost);
        }
    }
    function picked(): SyncObject[] {
        const values = status?.objects.filter(o => selected.has(objectKey(o.object))) ?? [];
        if (!values.length)
            throw new UiError(uiMessage("syncValidation.objectsRequired"));
        return values;
    }
    async function prepare(kind: "cloneSync" | "applySync"): Promise<void> {
        clearReview();
        await host.beforeAction();
        await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind, identity: identity(), objects: picked().map(o => o.object) } });
    }
    async function prepareAdoption(): Promise<void> {
        clearReview();
        const currentRoot = targetRoot(), paired = identity(), objects = picked().map(o => o.object);
        await host.beforeAction();
        const cut = await api<SyncAdoptionCut>("/api/sync/adoption-cut", { root: currentRoot, identity: paired, objects });
        await flow.prepare(freezeSyncAdoptionRequest(currentRoot, paired, cut, operationId()));
    }
    async function beginRevalidation(objects: ObjectKey[], initial: {
        object: ObjectKey;
        revision: string;
    }[]): Promise<void> {
        if (flow.pending)
            throw new UiError(uiMessage("syncValidation.originalOperationPending"));
        clearReview();
        const generation = reviewGeneration, currentRoot = targetRoot(), paired = identity();
        const cut = decodeSyncRevalidationCut(await api<unknown>("/api/sync/revalidation-cut", { root: currentRoot, identity: paired, objects }));
        if (generation !== reviewGeneration || currentRoot !== targetRoot() || !sameSyncIdentity(paired, identity()) || flow.pending)
            throw new UiError(uiMessage("syncValidation.reviewBasisChanged"));
        revalidation = mountSyncRevalidation(revalidationHost, currentRoot, paired, cut, initial, { request: api, run, beforeAction: host.beforeAction, id: operationId, prepare: request => flow.prepare(request, clearReview) });
    }
    async function inspectVersion(o: SyncObject, version: SyncVersion, kind: "head" | "base" | "history"): Promise<void> {
        const title = el("h3", () => `${uiTextValue(kind === "base" ? uiMessage("syncPanel.commonBase") : kind === "history" ? uiMessage("syncPanel.historyVersion") : uiMessage("syncPanel.version"))} ${uiTextValue(version.revision)}`);
        replaceUiChildren(inspectHost, title, el("pre", JSON.stringify({ parent: version.state.parent, name: version.state.name, retired: version.state.retired }, null, 2)));
        for (const [path, digest] of Object.entries(version.state.files)) {
            const section = el("details");
            section.append(el("summary", path), el("code", digest));
            const content = el("pre");
            section.append(content, button(displayMessage("syncPanel.readFile", { p0: uiDisplayParameter(path) }), async () => {
                const result = await api<{
                    text: string | null;
                    bytes: number;
                }>("/api/sync/file", { identity: identity(), object: o.object, revision: version.revision, path });
                setUiText(content, result.text ?? (displayMessage("syncPanel.filePreviewUnavailable", { p0: result.bytes })));
            }));
            inspectHost.append(section);
        }
        if (kind === "head" && binding && o.view.versions.length > 1) {
            const adoptBranch = button(uiMessage("syncPanel.previewBranchAdoption"), async () => {
                await host.beforeAction();
                await flow.prepare({ operationId: operationId(), root: targetRoot(), intent: { kind: "mergeSync", identity: identity(), resolutions: [{ object: o.object, revisions: o.view.versions.map(v => v.revision), state: version.state }], blobs: {} } });
            });
            adoptBranch.dataset.syncAdoptBranch = version.revision;
            inspectHost.append(el("p", uiMessage("syncPanel.branchAdoptionHelp")), adoptBranch);
        }
        if (kind === "head" && binding) {
            const review = button(uiMessage("syncPanel.reviewVersionSemantics"), () => beginRevalidation([o.object], [{ object: o.object, revision: version.revision }]));
            review.dataset.syncRevalidationStart = version.revision;
            inspectHost.append(review);
        }
    }
    async function inspectHistory(o: SyncObject, afterCommit?: string): Promise<void> {
        const result = decodeReceivedSyncVersions(await api<unknown>("/api/sync/history", { identity: identity(), object: o.object, afterCommit, limit: 25 }));
        const installed = binding?.installedVersions.find(([key]) => objectKey(key) === objectKey(o.object))?.[1];
        replaceUiChildren(historyHost, el("h3", displayMessage("syncPanel.objectHistoryTitle", { p0: uiDisplayParameter(objectKey(o.object)) })), el("p", uiMessage("syncPanel.receivedHistoryHelp")), el("p", result.coverage === "complete" ? uiMessage("syncPanel.receivedRecordsAdmitted") : uiMessage("syncPanel.receivedRecordsPartial")), button(uiMessage("syncPanel.reloadHistoryFirstPage"), () => inspectHistory(o)));
        if (!result.versions.length)
            historyHost.append(el("p", uiMessage("syncPanel.noReceivedHistory")));
        for (const version of result.versions) {
            const entry = el("details"), accepted = version.admission.status === "accepted";
            entry.append(el("summary", () => `${uiTextValue(version.revision)} · ${uiTextValue(accepted ? uiMessage("syncPanel.admittedStatus") : version.admission.status)}${uiTextValue(installed === version.revision ? uiMessage("syncPanel.installedSuffix") : uiMessage("syncPanel.notInstalledSuffix"))}`), el("pre", JSON.stringify({ commit: version.commit, parents: version.parents, before: version.before, admission: admissionFacts(version.admission), name: version.state.name, retired: version.state.retired }, null, 2)));
            if (version.admission.status === "rejected") entry.append(el("p", coreDescriptionUiText(version.admission.reason)));
            if (accepted)
                entry.append(button(displayMessage("syncPanel.viewHistoryVersion", { p0: uiDisplayParameter(version.revision) }), () => inspectVersion(o, version, "history")));
            historyHost.append(entry);
        }
        if (result.nextCommit)
            historyHost.append(button(uiMessage("syncPanel.nextHistoryPage"), () => inspectHistory(o, result.nextCommit!)));
        replaceUiChildren(inspectHost);
    }
    function render(): void {
        dialog.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = false; });
        rootField.input.disabled = !!flow.pending;
        pair.readOnly = !!binding || !!flow.pending;
        setUiText(info, binding ? displayMessage("syncPanel.replicaSummary", { p0: uiDisplayParameter(binding.replicaId), p1: uiDisplayParameter(binding.partial ? uiMessage("syncPanel.partialReplica") : uiMessage("syncPanel.completeReplica")), p2: binding.installedObjects.length, p3: binding.pendingPublications }) : uiMessage("syncPanel.notEnabledHelp"));
        replaceUiChildren(controls, button(uiMessage("syncPanel.readLocalStatus"), load), button(uiMessage("syncPanel.enable"), async () => {
            const path = targetRoot(), key = `weftext-sync-enable:${path}`;
            const id = localStorage.getItem(key) ?? operationId();
            localStorage.setItem(key, id);
            binding = decodeSyncBinding(await api<unknown>("/api/sync/enable", { root: path, operationId: id }));
            pair.value = JSON.stringify(binding.identity);
            await published();
            localStorage.removeItem(key);
        }), button(uiMessage("syncPanel.refreshSavedVersions"), published), button(uiMessage("syncPanel.receiveDirectory"), async () => {
            clearReview();
            status = decodeSyncStatus(await api<unknown>("/api/sync/receive", { identity: identity(), directory: transport.input.value }));
            selectedInitialized = false;
            replaceUiChildren(historyHost);
            replaceUiChildren(inspectHost);
        }), button(uiMessage("syncPanel.exportObjects"), async () => {
            await api("/api/sync/export", { root: targetRoot(), directory: transport.input.value, objects: picked().map(o => o.object) });
            await published();
        }), button(uiMessage("syncPanel.previewApply"), () => prepare("applySync")), button(uiMessage("syncPanel.previewClone"), () => prepare("cloneSync")), button(uiMessage("syncPanel.previewExternalAdoption"), prepareAdoption), button(uiMessage("syncPanel.reviewSelectedSemantics"), () => { const objects = picked(); return beginRevalidation(objects.map(o => o.object), objects.filter(o => o.view.versions.length === 1).map(o => ({ object: o.object, revision: o.view.versions[0].revision }))); }));
        const buttons = Array.from(controls.querySelectorAll("button"));
        buttons[1].disabled = !!binding || !!flow.pending;
        buttons[2].disabled = !binding || !!flow.pending;
        buttons[4].disabled = !binding || !!flow.pending;
        buttons[5].disabled = !binding || !status || !!flow.pending;
        buttons[6].disabled = !!binding || !status || !!flow.pending;
        buttons[0].disabled = !!flow.pending;
        buttons[3].disabled = !!flow.pending;
        buttons[7].disabled = !binding || !status || !!flow.pending;
        buttons[8].disabled = !binding || !status || !!flow.pending;
        replaceUiChildren(objectsHost);
        if (status) {
            if (!selectedInitialized) {
                selected.clear();
                for (const o of status.objects)
                    selected.add(objectKey(o.object));
                selectedInitialized = true;
            }
            objectsHost.append(el("h3", displayMessage("syncPanel.receivedSummary", { p0: status.commits.length, p1: status.heads.length })), el("p", uiMessage("syncPanel.receiveScopeHelp")));
            const unsettled = status.commits.filter(c => c.admission.status !== "accepted" || c.unsettledDependencies.length);
            if (unsettled.length) {
                const details = el("details");
                details.append(el("summary", displayMessage("syncPanel.unsettledCommitCount", { p0: unsettled.length })), el("p", uiMessage("syncPanel.historicalDependencyHelp")), el("pre", JSON.stringify(unsettled.map(commit => ({ ...commit, admission: admissionFacts(commit.admission) })), null, 2)));
                for (const commit of unsettled) if (commit.admission.status === "rejected") details.append(el("code", commit.id), el("p", coreDescriptionUiText(commit.admission.reason)));
                objectsHost.append(details);
            }
            const installed = new Map(binding?.installedVersions.map(([key, revision]) => [objectKey(key), revision]) ?? []);
            for (const o of status.objects) {
                const row = el("div"), label = el("label"), check = el("input");
                row.className = "sync-object";
                check.type = "checkbox";
                check.checked = selected.has(objectKey(o.object));
                check.disabled = !!flow.pending;
                check.onchange = () => {
                    if (check.checked)
                        selected.add(objectKey(o.object));
                    else
                        selected.delete(objectKey(o.object));
                };
                label.append(check, uiTextNode(`${o.view.versions.map(v => v.state.name).filter((n, i, a) => a.indexOf(n) === i).join(" / ") || o.object.kind} (${objectKey(o.object)})`));
                row.append(label);
                row.append(el("p", displayMessage("syncPanel.installedVersionSummary", { p0: uiDisplayParameter(installed.get(objectKey(o.object)) ?? uiMessage("syncPanel.notInstalled")), p1: uiDisplayParameter(o.view.versions.length > 1 ? uiMessage("syncPanel.conflictBranchesSuffix") : "") })));
                for (const v of o.view.versions)
                    row.append(button(displayMessage("syncPanel.viewVersion", { p0: uiDisplayParameter(v.revision) }), () => inspectVersion(o, v, "head")));
                for (const v of o.view.commonBases)
                    row.append(button(displayMessage("syncPanel.viewBase", { p0: uiDisplayParameter(v.revision) }), () => inspectVersion(o, v, "base")));
                row.append(button(uiMessage("syncPanel.viewAllReceivedHistory"), () => inspectHistory(o)));
                objectsHost.append(row);
            }
        }
        replaceUiChildren(pendingHost);
        const p = flow.pending;
        if (p) {
            pendingHost.append(el("h3", displayMessage("syncPanel.originalOperationTitle", { p0: uiDisplayParameter(p.status) })), el("code", p.request.operationId), el("p", displayMessage("syncPanel.targetSummary", { p0: uiDisplayParameter(p.request.root), p1: uiDisplayParameter(p.request.intent.kind) })));
            if (p.request.intent.kind === "adoptSyncExternal") {
                pendingHost.append(el("p", uiMessage("syncPanel.externalAdoptionHelp")), el("p", uiMessage("syncPanel.checkedInstalledObjects")), el("pre", JSON.stringify(p.request.intent.expectedInstalled, null, 2)));
                for (const file of p.request.intent.changes) {
                    const details = el("details");
                    details.append(el("summary", displayMessage("syncPanel.fileByteSummary", { p0: uiDisplayParameter(file.path), p1: file.after.length })));
                    try {
                        details.append(el("pre", new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array(file.after))));
                    }
                    catch {
                        details.append(el("p", uiMessage("syncPanel.nonUtf8OriginalBytes")));
                    }
                    pendingHost.append(details);
                }
                if (p.request.intent.removedMembers?.length) {
                    const list = el("ul");
                    for (const member of p.request.intent.removedMembers)
                        list.append(el("li", member.path));
                    pendingHost.append(el("p", uiMessage("syncPanel.externalRemovalsHelp")), list);
                }
            }
            if (p.request.intent.kind === "revalidateSync") {
                const intent = p.request.intent;
                pendingHost.append(el("p", uiMessage("syncPanel.revalidationOriginalHelp")));
                appendSyncRevalidationSummary(pendingHost, intent, status?.objects ?? []);
                for (const resolution of intent.resolutions)
                    for (const [path, digest] of Object.entries(resolution.state.files))
                        if (intent.blobs[digest]) {
                            const detail = el("details");
                            detail.append(el("summary", displayMessage("syncPanel.explicitRepairTitle", { p0: uiDisplayParameter(objectKey(resolution.object)), p1: uiDisplayParameter(path) })));
                            try {
                                detail.append(el("pre", new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(new Uint8Array(intent.blobs[digest]))));
                            }
                            catch {
                                detail.append(el("p", uiMessage("syncPanel.originalBinaryBytes")));
                            }
                            pendingHost.append(detail);
                        }
            }
            if (p.plan) {
                const list = el("ul");
                for (const c of p.plan.changes)
                    list.append(el("li", displayMessage("syncPanel.planChangeSummary", { p0: uiDisplayParameter(c.action), p1: uiDisplayParameter(c.path), p2: c.bytes })));
                pendingHost.append(list);
                for (const loss of p.plan.losses)
                    pendingHost.append(el("p", knowledgeLossMessage(loss)));
            }
            if (p.receipt?.documentError) pendingHost.append(el("p", apiErrorUiText(p.receipt.documentError)));
            pendingHost.append(button(uiMessage("syncPanel.inspectOriginal"), async () => { await flow.inspect(); await refreshCommitted(); }));
            if (!p.attempted && !p.plan)
                pendingHost.append(button(uiMessage("syncPanel.retryOriginalPrepare"), () => flow.retryPrepare()));
            if (p.status === "prepared")
                pendingHost.append(button(uiMessage("syncPanel.confirmPreview"), async () => { await host.beforeAction(); await flow.commit(); await refreshCommitted(); }));
            if (["committed", "resolved"].includes(p.status) || !p.attempted)
                pendingHost.append(button(["committed", "resolved"].includes(p.status) ? uiMessage("syncPanel.finishConfirmed") : uiMessage("syncPanel.discardLocalPreview"), () => { flow.forget(); replaceUiChildren(inspectHost); }));
            if (!p.attempted && !["committed", "resolved"].includes(p.status))
                pendingHost.append(el("p", uiMessage("syncPanel.discardPreviewHelp")));
            if (p.status === "prepared")
                pendingHost.append(button(uiMessage("syncPanel.cancelUnstartedProposal"), async () => { await flow.cancelPrepared(); await refreshCommitted(); }));
            if (["unknown", "recoveryPending"].includes(p.status))
                pendingHost.append(el("p", uiMessage("syncPanel.partialRecoveryHelp")));
        }
        if (flow.pending)
            inspectHost.querySelectorAll("button").forEach(e => {
                if (e.dataset.syncAdoptBranch || e.dataset.syncRevalidationStart)
                    e.disabled = true;
            });
        revalidation?.render(busy, !!flow.pending);
        if (busy)
            lock();
        remoteControls?.render();
    }
    const close = el("button", uiMessage("syncPanel.close"));
    close.type = "button";
    close.onclick = () => { clearReview(); remoteControls?.dispose(); disposeUiTree(dialog); dialog.close(); dialog.remove(); };
    dialog.oncancel = event => {
        if (busy)
            event.preventDefault();
        else {
            clearReview();
            remoteControls?.dispose();
            disposeUiTree(dialog);
            dialog.remove();
        }
    };
    rootField.input.oninput = () => { clearReview(); binding = null; status = null; replaceUiChildren(historyHost); replaceUiChildren(inspectHost); render(); };
    pair.oninput = () => { clearReview(); status = null; replaceUiChildren(historyHost); replaceUiChildren(inspectHost); render(); };
    dialog.append(el("h2", uiMessage("syncPanel.title")), el("p", uiMessage("syncPanel.replicaIsolationHelp")), rootField.label, pairLabel, transport.label, info, controls, statusLine, pendingHost, objectsHost, historyHost, inspectHost, revalidationHost, original.label, button(uiMessage("syncPanel.readOriginalOperation"), async () => {
        await flow.recover(original.input.value.trim());
        const p = flow.pending!;
        rootField.input.value = p.request.root;
        pair.value = JSON.stringify(p.request.intent.identity);
        replaceUiChildren(inspectHost);
        await load();
        await refreshCommitted();
    }), close);
    const remoteHost = el("section");
    dialog.insertBefore(remoteHost, original.label);
    remoteControls = mountRemoteSync(remoteHost, {
        identity, root: targetRoot, objects: () => picked().map(o => o.object), blocked: () => busy || !!flow.pending,
        received: async (paired) => {
            if (binding && !sameSyncIdentity(binding.identity, paired))
                return;
            if (pair.value) {
                try {
                    if (!sameSyncIdentity(identity(), paired))
                        return;
                }
                catch {
                    return;
                }
            }
            clearReview();
            pair.value = JSON.stringify(paired);
            status = decodeSyncStatus(await api<unknown>("/api/sync/status", { identity: paired }));
            selectedInitialized = false;
            if (binding)
                binding = decodeSyncBinding(await api<unknown>("/api/sync/binding", { root: targetRoot() }));
            render();
        },
    });
    document.body.append(dialog);
    dialog.showModal();
    render();
    if (rootField.input.value)
        await run(load);
}
