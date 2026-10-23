import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { api, operationId } from "./api";
import type { ObjectKey, SyncIdentity, SyncStatus } from "./sync-state";
import { decodeRemoteSyncProgress, decodeSyncStatus, objectKey, sameSyncIdentity, type RemoteSyncProgress } from "./sync-state";
export { decodeRemoteSyncProgress } from "./sync-state";
export type { RemoteSyncProgress } from "./sync-state";
import { descriptionUiText } from "./ui-diagnostics";
import { apiErrorUiText } from "./api-error";
import { disposeUiTree, joinUiText } from "./ui-locale";
interface PendingProgress { requestId: string; status: "running"; pendingLabel: UiText; identity?: SyncIdentity }
interface Host {
    identity(): SyncIdentity;
    root(): string;
    objects(): ObjectKey[];
    blocked(): boolean;
    received(identity: SyncIdentity): Promise<void>;
}
const storageKey = "weftext-remote-sync-job-v1";
export function mountRemoteSync(container: HTMLElement, host: Host): {
    render(): void;
    dispose(): void;
} {
    const title = document.createElement("h3");
    setUiText(title, uiMessage("remoteSync.title"));
    const help = document.createElement("p");
    setUiText(help, uiMessage("remoteSync.description"));
    function field(text: UiText, password = false): HTMLInputElement {
        const label = document.createElement("label");
        setUiText(label, text);
        const input = document.createElement("input");
        input.type = password ? "password" : "url";
        input.autocomplete = "off";
        input.spellcheck = false;
        label.append(input);
        container.append(label);
        return input;
    }
    container.append(title, help);
    const endpoint = field(uiMessage("remoteSync.serviceAddress")), credential = field(uiMessage("remoteSync.fullSyncCredential"), true);
    const list = document.createElement("section"), progress = document.createElement("p"), actions = document.createElement("div");
    progress.setAttribute("role", "status");
    setUiAttr(progress, "aria-label", uiMessage("remoteSync.transferProgress"));
    container.append(actions, progress, list);
    let busy = false, disposed = false, job: RemoteSyncProgress | PendingProgress | null = null, timer: ReturnType<typeof setTimeout> | undefined, selected = new Set<string>(), remote: SyncStatus | null = null;
    let lastDelivered = "";
    const connection = () => ({ endpoint: endpoint.value, token: credential.value, identity: host.identity() });
    const showError = (e: unknown) => { if (!disposed) setUiText(progress, errorUiText(e)); };
    async function run(f: () => Promise<void>): Promise<void> {
        if (busy || disposed)
            return;
        busy = true;
        render();
        try {
            await f();
        }
        catch (e) {
            showError(e);
        }
        finally {
            busy = false;
            render();
        }
    }
    function button(text: UiText, action: () => Promise<void>): HTMLButtonElement {
        const b = document.createElement("button");
        b.type = "button";
        setUiText(b, text);
        b.onclick = () => { void run(action); };
        return b;
    }
    async function receive(incoming: unknown): Promise<void> {
        if (disposed)
            return;
        const value = decodeRemoteSyncProgress(incoming);
        if (job && (value.requestId !== job.requestId || job.identity && !sameSyncIdentity(job.identity, value.identity))) throw new UiError(uiMessage("error.invalidResponse"));
        clearTimeout(timer);
        job = value;
        setUiText(progress, joinUiText([descriptionUiText(value), displayMessage("remoteSync.progressCounts", { p0: uiDisplayParameter(String(value.recordsDone)), p1: uiDisplayParameter(String(value.recordsTotal)), p2: uiDisplayParameter(String(value.bytesDone)), p3: uiDisplayParameter(String(value.bytesTotal)) }), value.error ? apiErrorUiText(value.error) : ""]));
        if (value.status === "complete" && value.identity && lastDelivered !== value.requestId) {
            await host.received(value.identity);
            if (disposed || job?.requestId !== value.requestId) return;
            lastDelivered = value.requestId;
        }
        if (value.status === "running")
            timer = setTimeout(() => { void poll(); }, 500);
        render();
    }
    async function poll(): Promise<void> {
        if (disposed || !job)
            return;
        try {
            const requestId = job.requestId, value = await api<unknown>("/api/sync/remote/job", { requestId });
            if (disposed || job?.requestId !== requestId) return;
            await receive(value);
        }
        catch (e) {
            showError(e);
            if (!disposed)
                timer = setTimeout(() => { void poll(); }, 2000);
        }
    }
    async function start(direction: "push" | "pull"): Promise<void> {
        if (host.blocked() || job?.status === "running")
            throw new UiError(uiMessage("remoteSync.finishPreparationOrTransfer"));
        const requestId = operationId();
        const objects = direction === "push" ? host.objects() : remote?.objects.filter(o => selected.has(objectKey(o.object))).map(o => o.object);
        if (direction === "pull" && (!remote || !objects?.length))
            throw new UiError(uiMessage("remoteSync.readDirectorySelectObject"));
        const captured = connection();
        if (remote && direction === "pull" && !sameSyncIdentity(captured.identity, remote.identity))
            throw new UiError(uiMessage("remoteSync.pairingChanged"));
        const request = { requestId, connection: captured, direction, root: direction === "push" ? host.root() : null, objects };
        // Only the job identifier is retained; credentials remain in memory.
        localStorage.setItem(storageKey, requestId);
        job = { requestId, status: "running", identity: captured.identity, pendingLabel: uiMessage("remoteSync.confirmingOriginalTransfer") };
        setUiText(progress, job.pendingLabel);
        try {
            await receive(await api<unknown>("/api/sync/remote/start", request));
        }
        catch (e) {
            showError(e);
            if (!disposed) timer = setTimeout(() => { void poll(); }, 500);
            throw e;
        }
    }
    const inspect = button(uiMessage("remoteSync.readRemoteCatalog"), async () => {
        remote = null;
        disposeUiTree(list);
        list.replaceChildren();
        setUiText(progress, uiMessage("remoteSync.readingRemoteCatalog"));
        const captured = connection(), incoming = await api<unknown>("/api/sync/remote/inspect", captured);
        if (disposed) return;
        const current = connection();
        if (captured.endpoint !== current.endpoint || captured.token !== current.token || !sameSyncIdentity(captured.identity, current.identity)) throw new UiError(uiMessage("remoteSync.pairingChanged"));
        const decoded = decodeSyncStatus(incoming);
        if (!sameSyncIdentity(decoded.identity, captured.identity)) throw new UiError(uiMessage("remoteSync.pairingChanged"));
        remote = decoded;
        selected = new Set(remote.objects.map(o => objectKey(o.object)));
        list.replaceChildren();
        for (const item of remote.objects) {
            const label = document.createElement("label"), check = document.createElement("input");
            check.type = "checkbox";
            check.checked = true;
            check.onchange = () => {
                if (check.checked)
                    selected.add(objectKey(item.object));
                else
                    selected.delete(objectKey(item.object));
            };
            label.append(check, uiTextNode(displayMessage("remoteSync.remoteObject", { p0: uiDisplayParameter(item.view.versions.map(v => v.state.name).join(" / ") || item.object.kind), p1: uiDisplayParameter(objectKey(item.object)) })));
            list.append(label);
        }
        setUiText(progress, displayMessage("remoteSync.remoteObjectCount", { p0: remote.objects.length }));
    });
    const push = button(uiMessage("remoteSync.uploadSelectedLocal"), () => start("push")), pull = button(uiMessage("remoteSync.receiveSelectedRemote"), () => start("pull"));
    const stop = button(uiMessage("remoteSync.stopRemoteTransfer"), async () => {
        if (job) {
            clearTimeout(timer);
            await receive(await api<unknown>("/api/sync/remote/stop", { requestId: job.requestId }));
        }
    });
    actions.append(inspect, push, pull, stop);
    endpoint.oninput = credential.oninput = () => { remote = null; disposeUiTree(list); list.replaceChildren(); };
    function render(): void {
        if (disposed) return;
        const running = job?.status === "running";
        endpoint.disabled = credential.disabled = busy || running;
        inspect.disabled = push.disabled = pull.disabled = busy || running || host.blocked();
        stop.disabled = busy || !running;
        list.querySelectorAll("input").forEach(e => { e.disabled = busy || running; });
    }
    const saved = localStorage.getItem(storageKey);
    if (saved) {
        job = { requestId: saved, status: "running", pendingLabel: uiMessage("remoteSync.readingOriginalTransferStatus") };
        setUiText(progress, job.pendingLabel);
        void poll();
    }
    render();
    return { render, dispose: () => { disposed = true; clearTimeout(timer); credential.value = ""; job = null; remote = null; selected.clear(); disposeUiTree(container); } };
}
