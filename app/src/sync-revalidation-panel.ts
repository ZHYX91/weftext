import { disposeUiTree, replaceUiChildren, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { objectKey, type ObjectKey, type SyncIdentity, type SyncObject, type SyncRequest, type SyncRevalidation, type SyncRevalidationCut, type SyncRevalidationReview } from "./sync-state";
import { SyncRevalidationSession, type SyncExactText } from "./sync-revalidation";
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] { const node = document.createElement(tag); setUiText(node, text); return node; }
const kindNames: Record<ObjectKey["kind"], UiText> = { node: uiMessage("queryColumn.note"), directory: uiMessage("nodeContents.directoryKind"), rootDirectory: uiMessage("syncRevalidation.kindLibraryBookmark"), schema: uiMessage("syncRevalidation.kindPropertyDefinitions"), settings: uiMessage("error.api.hostedLibrary.settingsRoleTitle"), rules: uiMessage("error.api.hostedLibrary.rulesRoleTitle"), tags: uiMessage("field.tags") };
function nameOf(object: ObjectKey, objects: SyncObject[], resolutions: SyncRevalidation[] = []): UiText {
    if (!["node", "directory", "rootDirectory"].includes(object.kind))
        return kindNames[object.kind];
    return resolutions.find(r => objectKey(r.object) === objectKey(object))?.state.name || objects.find(o => objectKey(o.object) === objectKey(object))?.view.versions.map(v => v.state.name).filter((v, i, all) => all.indexOf(v) === i).join(" / ") || kindNames[object.kind];
}
function short(revision: string, candidates: string[] = []): string {
    let length = 12;
    while (length < revision.length && candidates.some(other => other !== revision && other.slice(0, length) === revision.slice(0, length)))
        length++;
    return revision.length > length ? `${revision.slice(0, length)}…` : revision;
}
function decisionText(item: SyncRevalidation, objects: SyncObject[]): UiText {
    const revision = item.decision.kind === "preserve" ? item.decision.revision : item.decision.baseRevision;
    if (item.decision.kind === "preserve")
        return displayMessage("syncRevalidation.keepOriginalVersion", { p0: uiDisplayParameter(short(revision, item.revisions)) });
    const original = objects.find(o => objectKey(o.object) === objectKey(item.object))?.view.versions.find(v => v.revision === revision)?.state;
    const changed = original ? Object.keys(item.state.files).filter(path => original.files[path] !== item.state.files[path]) : [];
    return displayMessage("syncRevalidation.useRepairedContent", { p0: uiDisplayParameter(changed.length ? `：${changed.join("、")}` : ""), p1: uiDisplayParameter(short(revision, item.revisions)) });
}
export function appendSyncRevalidationSummary(container: HTMLElement, review: Pick<SyncRevalidationReview, "expectedInstalled" | "reviewedVersions" | "resolutions">, objects: SyncObject[]): void {
    const candidates = [...review.expectedInstalled, ...review.reviewedVersions].flatMap(dep => dep.revisions), versionText = (revision: string): string => short(revision, candidates);
    const table = el("table"), header = el("tr");
    for (const text of [uiMessage("syncRevalidation.object"), uiMessage("syncRevalidation.decision"), uiMessage("syncRevalidation.localOriginalVersion"), uiMessage("syncRevalidation.reviewedVersion")] as const)
        header.append(el("th", text));
    table.append(header);
    for (const received of review.reviewedVersions) {
        const row = el("tr"), resolution = review.resolutions.find(r => objectKey(r.object) === objectKey(received.object)), installed = review.expectedInstalled.find(v => objectKey(v.object) === objectKey(received.object));
        row.append(el("td", nameOf(received.object, objects, review.resolutions)), el("td", resolution ? decisionText(resolution, objects) : uiMessage("syncRevalidation.checkWithTransactionOrDependencies")), el("td", installed ? installed.revisions.map(versionText).join("、") : uiMessage("syncPanel.notInstalled")), el("td", received.revisions.map(versionText).join("、")));
        table.append(row);
    }
    container.append(table);
    for (const item of review.resolutions)
        container.append(el("p", displayMessage("syncRevalidation.semanticDependencies", { p0: uiDisplayParameter(nameOf(item.object, objects, review.resolutions)), p1: uiDisplayParameter(item.reviewedDependencies.length ? () => item.reviewedDependencies.map(dep => `${uiTextValue(nameOf(dep.object, objects, review.resolutions))}（${dep.revisions.map(versionText).join("、")}）`).join("；") : uiMessage("syncRevalidation.noOtherObjects")) })));
    const technical = el("details");
    technical.append(el("summary", uiMessage("syncRevalidation.technicalVersionsRequest")), el("pre", JSON.stringify({ expectedInstalled: review.expectedInstalled, reviewedVersions: review.reviewedVersions, resolutions: review.resolutions }, null, 2)));
    container.append(technical);
}
interface Host {
    request<T>(path: string, body: unknown): Promise<T>;
    run(action: () => Promise<void>): Promise<void>;
    prepare(request: SyncRequest): Promise<void>;
    beforeAction(): Promise<void>;
    id(): string;
}
export function mountSyncRevalidation(container: HTMLElement, root: string, identity: SyncIdentity, cut: SyncRevalidationCut, initial: {
    object: ObjectKey;
    revision: string;
}[], host: Host): {
    render(busy: boolean, pending: boolean): void;
    dispose(): void;
} {
    const session = new SyncRevalidationSession(root, identity, cut, host.id), summary = el("section"), choices = el("section"), actions = el("div"), pendingEdits = new Set<HTMLTextAreaElement>();
    let disposed = false, blocked = false;
    for (const selected of initial)
        session.select(selected.object, selected.revision);
    container.dataset.syncRevalidation = "review";
    container.append(el("h3", uiMessage("syncRevalidation.title")), el("p", uiMessage("syncRevalidation.description")), el("p", uiMessage("syncRevalidation.candidateDescription")), choices, actions, summary);
    if (cut.coverage !== "complete")
        container.insertBefore(el("p", uiMessage("syncRevalidation.candidateHistoryMissingPieces")), choices);
    function button(text: UiText, action: () => Promise<void>): HTMLButtonElement {
        const b = el("button", text);
        b.type = "button";
        b.onclick = () => {
            if (!blocked && !disposed)
                void host.run(action);
        };
        return b;
    }
    function clearSummary(): void { replaceUiChildren(summary); delete summary.dataset.syncRevalidationSummary; confirm.disabled = true; read.disabled = blocked || pendingEdits.size > 0; }
    const read = button(uiMessage("syncRevalidation.readActualDependencies"), async () => {
        if (pendingEdits.size)
            throw new UiError(uiMessage("syncRevalidation.resolveUnconfirmedRepairInput"));
        clearSummary();
        read.disabled = true;
        const accepted = await session.inspect(input => host.request<SyncRevalidationReview>("/api/sync/revalidation-review", input));
        if (disposed || !accepted)
            throw new UiError(uiMessage("syncRevalidation.reviewSelectionChanged"));
        renderSummary();
    });
    read.dataset.syncRevalidationAction = "review";
    const confirm = button(uiMessage("syncRevalidation.confirmDependenciesPreview"), async () => { await host.beforeAction(); await host.prepare(session.request()); });
    confirm.dataset.syncRevalidationAction = "prepare";
    actions.append(read, confirm);
    for (const item of cut.objects) {
        const key = objectKey(item.object), row = el("details"), selected = session.selection(item.object), label = el("label", uiMessage("syncRevalidation.decision")), select = el("select"), files = el("section"), decision = el("p");
        row.dataset.syncRevalidationObject = key;
        row.open = !!selected;
        row.append(el("summary", () => `${uiTextValue(nameOf(item.object, cut.objects))} · ${uiTextValue(kindNames[item.object.kind])}`), el("p", displayMessage("syncRevalidation.receivedBranchCount", { p0: item.view.versions.length })));
        const none = el("option", uiMessage("syncRevalidation.dependencyCandidateOnly"));
        none.value = "";
        select.append(none);
        for (const version of item.view.versions) {
            const option = el("option", displayMessage("syncRevalidation.keepCompleteVersion", { p0: uiDisplayParameter(short(version.revision, item.view.versions.map(v => v.revision))) }));
            option.value = version.revision;
            setUiAttr(option, "title", version.revision);
            select.append(option);
        }
        select.value = selected?.decision.kind === "preserve" ? selected.decision.revision : "";
        select.dataset.syncRevalidationChoice = key;
        label.append(select);
        row.append(label, decision, files);
        const original = el("details");
        original.append(el("summary", uiMessage("syncRevalidation.technicalCommitDependencies")), el("pre", JSON.stringify({ object: item.object, revisions: item.view.versions.map(v => v.revision), originalDependencies: item.originalDependencies, unsettledDependencies: item.unsettledDependencies }, null, 2)));
        row.append(original);
        select.onchange = () => {
            for (const input of files.querySelectorAll("textarea"))
                pendingEdits.delete(input);
            if (select.value)
                session.select(item.object, select.value);
            else
                session.unselect(item.object);
            clearSummary();
            renderFiles();
        };
        function renderFiles(): void {
            replaceUiChildren(files);
            const chosen = session.selection(item.object);
            setUiText(decision, chosen ? displayMessage("syncRevalidation.selectedDecision", { p0: uiDisplayParameter(decisionText(chosen, cut.objects)) }) : uiMessage("syncRevalidation.unselectedNoDecision"));
            if (!chosen)
                return;
            files.append(el("p", uiMessage("syncRevalidation.keepBranchDescription")));
            for (const path of Object.keys(chosen.state.files)) {
                const section = el("details"), feedback = el("p"), content = el("section");
                section.append(el("summary", path));
                const load = button(displayMessage("syncRevalidation.readOriginalFile", { p0: uiDisplayParameter(path) }), async () => {
                    const earlier = content.querySelector("textarea");
                    if (earlier && pendingEdits.has(earlier))
                        throw new UiError(uiMessage("syncRevalidation.resolveFileRepairInput"));
                    const revision = chosen.decision.kind === "preserve" ? chosen.decision.revision : chosen.decision.baseRevision;
                    const exact = await host.request<SyncExactText>("/api/sync/file", { identity, object: item.object, revision, path });
                    if (disposed)
                        return;
                    const source = await session.verifyText(item.object, exact);
                    if (disposed || select.value !== revision || !container.contains(section))
                        throw new UiError(uiMessage("syncRevalidation.fileReadBasisChanged"));
                    replaceUiChildren(content);
                    const textLabel = el("label", displayMessage("syncRevalidation.repairContent", { p0: uiDisplayParameter(path) })), text = el("textarea");
                    text.rows = 10;
                    text.spellcheck = false;
                    const appliedSource = session.repairedText(item.object, path) ?? source;
                    text.value = appliedSource;
                    text.dataset.syncRevalidationRepair = path;
                    textLabel.append(text);
                    const apply = button(uiMessage("syncRevalidation.adoptFileRepair"), async () => {
                        await session.repairText(item.object, path, text.value);
                        pendingEdits.delete(text);
                        apply.disabled = true;
                        setUiText(decision, displayMessage("syncRevalidation.selectedDecision", { p0: uiDisplayParameter(decisionText(session.selection(item.object)!, cut.objects)) }));
                        setUiText(feedback, session.selection(item.object)?.decision.kind === "repair" ? uiMessage("syncRevalidation.repairAdoptedRereadDependencies") : uiMessage("syncRevalidation.sameAsOriginalKeepRevision"));
                        clearSummary();
                    });
                    apply.dataset.syncRevalidationAction = "repair";
                    apply.disabled = true;
                    text.oninput = () => { session.touch(); pendingEdits.add(text); setUiText(feedback, uiMessage("syncRevalidation.repairUnadopted")); clearSummary(); apply.disabled = blocked; };
                    const discard = button(uiMessage("syncRevalidation.discardUnadoptedInput"), async () => { text.value = session.repairedText(item.object, path) ?? source; pendingEdits.delete(text); apply.disabled = true; session.touch(); clearSummary(); });
                    content.append(textLabel, apply, discard);
                });
                section.append(load, feedback, content);
                files.append(section);
            }
        }
        renderFiles();
        choices.append(row);
    }
    function renderSummary(): void {
        const review = session.review;
        replaceUiChildren(summary);
        if (!review)
            return;
        summary.dataset.syncRevalidationSummary = "ready";
        summary.append(el("h4", uiMessage("syncRevalidation.actualReviewBasis")), el("p", uiMessage("syncRevalidation.checkReviewBeforePreview")));
        appendSyncRevalidationSummary(summary, review, cut.objects);
        for (const target of review.annotationTargets)
            summary.append(el("p", displayMessage("syncRevalidation.annotationTargetState", { p0: uiDisplayParameter(target.annotationId), p1: uiDisplayParameter(target.state === "exact" ? uiMessage("syncRevalidation.targetVerified") : target.state === "unavailable" ? uiMessage("syncRevalidation.discussionRetainedTargetUnavailable") : target.state) })), el("code", target.reason));
        confirm.disabled = blocked || pendingEdits.size > 0;
    }
    function render(busy: boolean, pending: boolean): void {
        blocked = busy || pending || disposed;
        container.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement | HTMLSelectElement>("input,button,textarea,select").forEach(e => { e.disabled = blocked; });
        read.disabled = blocked || pendingEdits.size > 0;
        confirm.disabled = blocked || pendingEdits.size > 0 || !session.review;
        // File repair controls remain explicit, even after the outer sync panel
        // unlocks all controls following a read-only request.
        for (const section of container.querySelectorAll<HTMLElement>("section")) {
            const text = section.querySelector<HTMLTextAreaElement>(":scope > label > textarea"), apply = section.querySelector<HTMLButtonElement>(":scope > button[data-sync-revalidation-action='repair']");
            if (text && apply)
                apply.disabled = blocked || !pendingEdits.has(text);
        }
    }
    render(false, false);
    return { render, dispose: () => { disposed = true; session.invalidate(); pendingEdits.clear(); disposeUiTree(container); } };
}
