import { api, operationId } from "./api";
import type { ObjectKey, SyncIdentity, SyncStatus } from "./sync-state";
import { objectKey, sameSyncIdentity } from "./sync-state";
interface Progress { requestId: string; identity?: SyncIdentity; status: string; recordsDone?: number; recordsTotal?: number; bytesDone?: number; bytesTotal?: number; message: string }
interface Host {
  identity(): SyncIdentity;
  root(): string;
  objects(): ObjectKey[];
  blocked(): boolean;
  received(identity: SyncIdentity): Promise<void>;
}
const storageKey = "weftext-remote-sync-job-v1";
export function mountRemoteSync(container: HTMLElement, host: Host): { render(): void; dispose(): void } {
  const title = document.createElement("h3"); title.textContent = "远程托管同步";
  const help = document.createElement("p"); help.textContent = "使用管理员授予的同所有者完整同步凭据。传输只接收历史，应用仍需本机预览。凭据不写入浏览器存储。关闭窗口不会停止正在进行的传输。";
  function field(text: string, password = false): HTMLInputElement {
    const label = document.createElement("label"); label.textContent = text; const input = document.createElement("input"); input.type = password ? "password" : "url";
    input.autocomplete = "off"; input.spellcheck = false; label.append(input); container.append(label); return input;
  }
  container.append(title, help);
  const endpoint = field("远端服务地址"), credential = field("完整同步凭据", true);
  const list = document.createElement("section"), progress = document.createElement("p"), actions = document.createElement("div");
  progress.setAttribute("role", "status"); progress.setAttribute("aria-label", "远程传输进度");
  container.append(actions, progress, list);
  let busy = false, disposed = false, job: Progress | null = null, timer: ReturnType<typeof setTimeout> | undefined, selected = new Set<string>(), remote: SyncStatus | null = null;
  let lastDelivered = "";
  const connection = () => ({ endpoint: endpoint.value, token: credential.value, identity: host.identity() });
  const showError = (e: unknown) => { progress.textContent = e instanceof Error ? e.message : String(e); };
  async function run(f: () => Promise<void>): Promise<void> {
    if (busy) return; busy = true; render();
    try { await f(); } catch (e) { showError(e); } finally { busy = false; render(); }
  }
  function button(text: string, action: () => Promise<void>): HTMLButtonElement {
    const b = document.createElement("button"); b.type = "button"; b.textContent = text; b.onclick = () => { void run(action); }; return b;
  }
  async function receive(value: Progress): Promise<void> {
    if (disposed) return;
    job = value;
    progress.textContent = `${value.message}${value.recordsTotal !== undefined ? ` · ${value.recordsDone}/${value.recordsTotal} 条记录 · ${value.bytesDone}/${value.bytesTotal} 字节` : ""}`;
    if (value.status === "complete" && value.identity && lastDelivered !== value.requestId) { await host.received(value.identity); lastDelivered = value.requestId; }
    if (value.status === "running") timer = setTimeout(() => { void poll(); }, 500);
    render();
  }
  async function poll(): Promise<void> {
    if (disposed || !job) return;
    try { await receive(await api<Progress>("/api/sync/remote/job", { requestId: job.requestId })); }
    catch (e) { showError(e); if (!disposed) timer = setTimeout(() => { void poll(); }, 2000); }
  }
  async function start(direction: "push" | "pull"): Promise<void> {
    if (host.blocked() || job?.status === "running") throw new Error("请先完成当前准备或传输。");
    const requestId = operationId();
    const objects = direction === "push" ? host.objects() : remote?.objects.filter(o => selected.has(objectKey(o.object))).map(o => o.object);
    if (direction === "pull" && (!remote || !objects?.length)) throw new Error("请先读取远端目录并选择对象。");
    const captured = connection();
    if (remote && direction === "pull" && !sameSyncIdentity(captured.identity, remote.identity)) throw new Error("配对已变化，请重新读取远端目录。");
    const request = { requestId, connection: captured, direction, root: direction === "push" ? host.root() : null, objects };
    // Only the job identifier is retained; credentials remain in memory.
    localStorage.setItem(storageKey, requestId);
    job = { requestId, status: "running", message: "正在确认原传输请求" };
    progress.textContent = job.message;
    try { await receive(await api<Progress>("/api/sync/remote/start", request)); }
    catch (e) { showError(e); timer = setTimeout(() => { void poll(); }, 500); throw e; }
  }
  const inspect = button("读取远端目录", async () => {
    remote = null; list.replaceChildren();
    progress.textContent = "正在读取远端目录…";
    remote = await api<SyncStatus>("/api/sync/remote/inspect", connection()); selected = new Set(remote.objects.map(o => objectKey(o.object)));
    list.replaceChildren();
    for (const item of remote.objects) {
      const label = document.createElement("label"), check = document.createElement("input"); check.type = "checkbox"; check.checked = true;
      check.onchange = () => { if (check.checked) selected.add(objectKey(item.object)); else selected.delete(objectKey(item.object)); };
      label.append(check, document.createTextNode(`远端：${item.view.versions.map(v => v.state.name).join(" / ") || item.object.kind} (${objectKey(item.object)})`)); list.append(label);
    }
    progress.textContent = `远端 ${remote.objects.length} 个对象；尚未接收或应用到本机。`;
  });
  const push = button("上传所选本机对象", () => start("push")), pull = button("接收所选远端对象", () => start("pull"));
  const stop = button("停止远端传输", async () => { if (job) { clearTimeout(timer); await receive(await api<Progress>("/api/sync/remote/stop", { requestId: job.requestId })); } });
  actions.append(inspect, push, pull, stop);
  endpoint.oninput = credential.oninput = () => { remote = null; list.replaceChildren(); };
  function render(): void {
    const running = job?.status === "running";
    endpoint.disabled = credential.disabled = busy || running;
    inspect.disabled = push.disabled = pull.disabled = busy || running || host.blocked();
    stop.disabled = busy || !running;
    list.querySelectorAll("input").forEach(e => { e.disabled = busy || running; });
  }
  const saved = localStorage.getItem(storageKey);
  if (saved) { job = { requestId: saved, status: "running", message: "正在读取原传输状态" }; void poll(); }
  render();
  return { render, dispose: () => { disposed = true; clearTimeout(timer); credential.value = ""; } };
}
