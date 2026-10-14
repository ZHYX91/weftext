export class ApiError extends Error {
  constructor(public code: string, message: string, public details?: unknown, public currentSource?: string) { super(message); }
}

const fragment = new URLSearchParams(location.hash.slice(1));
const suppliedToken = fragment.get("token");
if (suppliedToken) {
  sessionStorage.setItem("weftext-token", suppliedToken);
  history.replaceState(null, "", location.pathname + location.search);
}

export async function api<T>(path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Weftext-Token": sessionStorage.getItem("weftext-token") ?? "",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data: unknown = await response.json();
  if (!response.ok || (data && typeof data === "object" && "error" in data)) {
    const error = (data as { error?: { code?: string; message?: string; details?: unknown; currentSource?: string } }).error;
    const details = error?.details as { currentSource?: string } | undefined;
    throw new ApiError(error?.code ?? "requestFailed", error?.message ?? `请求失败 (${response.status})`, error?.details, error?.currentSource ?? details?.currentSource);
  }
  return data as T;
}

export const operationId = (): string => crypto.randomUUID();

export async function apiBlob(path: string, body: unknown): Promise<Blob> {
  const response = await fetch(path, { method: "POST", headers: { "Content-Type": "application/json", "X-Weftext-Token": sessionStorage.getItem("weftext-token") ?? "" }, body: JSON.stringify(body) });
  if (!response.ok) {
    const data = await response.json() as { error?: { code?: string; message?: string; details?: unknown } };
    throw new ApiError(data.error?.code ?? "resourceReadFailed", data.error?.message ?? "资源读取失败", data.error?.details);
  }
  return response.blob();
}
