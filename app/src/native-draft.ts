/** Fixed native transport into the same local Core. It has no URI, path,
 * arbitrary operation or browser persistence acknowledgement in its protocol. */
type Binding = { surfaceId: string; surfaceEpoch: string };
const pending = new Map<string, { bytes: number; resolve: (body: unknown) => void; reject: (error: Error) => void }>();
let pendingBytes = 0;
let binding: Binding | undefined;
let installed: object | undefined;
export function bindNativeDraftSurface(next: Binding): void {
  if (binding && (binding.surfaceId !== next.surfaceId || binding.surfaceEpoch !== next.surfaceEpoch)) throw new Error("The live native draft surface cannot be rebound");
  binding = { ...next };
  const port = window.chrome?.webview; if (!port || installed === port) return;
  if (pending.size) throw new Error("The original native draft requests have not settled");
  installed = port;
  port.addEventListener("message", event => {
    const reply = event.data as { type?: string; surfaceId?: string; surfaceEpoch?: string; requestId?: string; body?: unknown } | null;
    if (!reply || reply.type !== "core-draft-result" || reply.surfaceId !== binding?.surfaceId || reply.surfaceEpoch !== binding?.surfaceEpoch || typeof reply.requestId !== "string") return;
    const request = pending.get(reply.requestId); if (!request) return;
    pending.delete(reply.requestId); pendingBytes -= request.bytes; request.resolve(reply.body);
  });
}
export function nativeDraftRequest(path: string, body: unknown): Promise<unknown> | undefined {
  const kind = path === "/api/draft" ? "persist" : path === "/api/draft/resume" ? "resume" : path === "/api/desktop/document-flush" ? "flush" : undefined;
  if (!kind || typeof window === "undefined") return undefined;
  const port = window.chrome?.webview;
  if (!kind || !port || !binding) return undefined;
  if (!body || typeof body !== "object") return Promise.reject(new Error("The native draft request has no typed body"));
  const source = (body as { source?: unknown; recoveredSource?: unknown })[kind === "resume" ? "recoveredSource" : "source"];
  if (kind !== "flush") {
    if (typeof source !== "string") return Promise.reject(new Error("The native draft source is absent"));
    for (let index = 0; index < source.length; index++) {
      const unit = source.charCodeAt(index);
      if (unit >= 0xd800 && unit <= 0xdbff) { const low = source.charCodeAt(++index); if (!(low >= 0xdc00 && low <= 0xdfff)) return Promise.reject(new Error("This raw UTF16 input cannot be represented by the document Core; it remains in the editor")); }
      else if (unit >= 0xdc00 && unit <= 0xdfff) return Promise.reject(new Error("This raw UTF16 input cannot be represented by the document Core; it remains in the editor"));
    }
    if (new TextEncoder().encode(source).length > 2 * 1024 * 1024) return Promise.reject(new Error("The source exceeds the document Core byte budget; it remains in the editor"));
  }
  if (pending.size >= 8) return Promise.reject(new Error("The native draft transport is busy; current input remains in the editor"));
  const requestId = crypto.randomUUID();
  const envelope = { type: "draft-call", ...binding, requestId, request: { kind, body } };
  const bytes = new TextEncoder().encode(JSON.stringify(envelope)).length;
  if (bytes > 16 * 1024 * 1024 || pendingBytes + bytes > 16 * 1024 * 1024) return Promise.reject(new Error("The complete native draft request exceeds its transport budget; input remains in the editor"));
  return new Promise((resolve, reject) => {
    pending.set(requestId, { bytes, resolve, reject }); pendingBytes += bytes;
    try { port.postMessage(envelope); }
    catch { pending.delete(requestId); pendingBytes -= bytes; reject(new Error("The native draft transport is unavailable; current input remains in the editor")); }
    // No timeout guesses whether an accepted Core write occurred. The same
    // ordered producer waits for its real result; unknown never authorizes close.
  });
}
