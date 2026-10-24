import { ApiError } from "./api-error";
import { message } from "./ui-locale";

let requests = 0;
let frozen = false;
/** Transport/decoding is one producer lifetime. Actual callers separately own
 * any subsequent asynchronous application and original-operation delivery. */
export function nativeApiIdle(): boolean { return requests === 0; }
export function freezeNativeApi(): boolean {
  if (frozen || requests !== 0) return false;
  frozen = true;
  return true;
}
export function thawNativeApi(): void { frozen = false; }
export function beginNativeRequest(): () => void {
  if (frozen) throw new ApiError("nativeSurfaceFrozen", message("desktop.unsavedClose"));
  requests++;
  let finished = false;
  return () => { if (!finished) { finished = true; requests--; } };
}
