import { cancellableQuery } from "./cancellable-query";
import { ApiError, decodeApiError } from "./api-error";
import { message } from "./ui-locale";
import { readStrictJsonResponse, StrictJsonError } from "./strict-json";
export { ApiError } from "./api-error";
import { beginNativeRequest } from "./native-api-gate";
export { nativeApiIdle, freezeNativeApi, thawNativeApi } from "./native-api-gate";

const fragment = new URLSearchParams(location.hash.slice(1));
const suppliedToken = fragment.get("token");
if (suppliedToken) {
  sessionStorage.setItem("weftext-token", suppliedToken);
  history.replaceState(null, "", location.pathname + location.search);
}

async function send(path: string, body?: unknown, runId?: string): Promise<Response> {
  try {
    return await fetch(path, { method: body === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json", "X-Weftext-Token": sessionStorage.getItem("weftext-token") ?? "", ...(runId ? { "X-Weftext-Query-Run": runId } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
  } catch (error) {
    throw new ApiError("networkRequest", message("error.networkRequest"), [{ origin: "network", text: error instanceof Error ? error.message : String(error) }]);
  }
}

async function json(response: Response): Promise<unknown> {
  try { return await readStrictJsonResponse(response); }
  catch (error) { throw new ApiError("invalidResponse", message("error.invalidResponse"), error instanceof StrictJsonError ? [] : [{ origin: "json", text: error instanceof Error ? error.message : String(error) }]); }
}

function responseError(data: unknown, status: number): ApiError {
  const decoded = data && typeof data === "object" && !Array.isArray(data) && Object.keys(data).length === 1 && Object.hasOwn(data, "error") ? decodeApiError((data as { error: unknown }).error) : undefined;
  return decoded ?? new ApiError("invalidResponse", message("error.invalidResponse"), [], undefined, [], status);
}

export async function api<T>(path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  if (signal) {
    return cancellableQuery(signal, () => api("/api/query/prepare", {}),
      id => apiRequest<T>(path, body, id), runId => api("/api/query/cancel", { runId }));
  }
  return apiRequest<T>(path, body);
}

async function apiRequest<T>(path: string, body?: unknown, runId?: string): Promise<T> {
  const finish = beginNativeRequest();
  try {
  const native = nativeDraftRequest(path, body);
  if (native) {
    const data: unknown = await native;
    if (data && typeof data === "object" && Object.hasOwn(data, "error")) throw responseError(data, 200);
    return data as T;
  }
  const response = await send(path, body, runId), data = await json(response);
  if (!response.ok || data && typeof data === "object" && !Array.isArray(data) && Object.keys(data).length === 1 && Object.hasOwn(data, "error")) throw responseError(data, response.status);
  return data as T;
  } finally { finish(); }
}

export const operationId = (): string => crypto.randomUUID();

export async function apiBlob(path: string, body: unknown): Promise<Blob> {
  const finish = beginNativeRequest();
  try {
  const response = await send(path, body);
  if (!response.ok) throw responseError(await json(response), response.status);
  return await response.blob();
  } finally { finish(); }
}
import { nativeDraftRequest } from "./native-draft";
