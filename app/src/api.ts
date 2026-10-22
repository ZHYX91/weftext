import { ApiError, decodeApiError } from "./api-error";
import { message } from "./ui-locale";
import { readStrictJsonResponse, StrictJsonError } from "./strict-json";
export { ApiError } from "./api-error";

const fragment = new URLSearchParams(location.hash.slice(1));
const suppliedToken = fragment.get("token");
if (suppliedToken) {
  sessionStorage.setItem("weftext-token", suppliedToken);
  history.replaceState(null, "", location.pathname + location.search);
}

async function send(path: string, body?: unknown): Promise<Response> {
  try {
    return await fetch(path, { method: body === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json", "X-Weftext-Token": sessionStorage.getItem("weftext-token") ?? "" }, body: body === undefined ? undefined : JSON.stringify(body) });
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

export async function api<T>(path: string, body?: unknown): Promise<T> {
  const response = await send(path, body), data = await json(response);
  if (!response.ok || data && typeof data === "object" && Object.hasOwn(data, "error")) throw responseError(data, response.status);
  return data as T;
}

export const operationId = (): string => crypto.randomUUID();

export async function apiBlob(path: string, body: unknown): Promise<Blob> {
  const response = await send(path, body);
  if (!response.ok) throw responseError(await json(response), response.status);
  return response.blob();
}
