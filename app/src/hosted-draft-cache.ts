import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { HostedDraft } from "./hosted-state";
import { RecoveryTextCache, type StoragePort } from "./recovery-text-cache";
export class HostedDraftCache extends RecoveryTextCache<HostedDraft> {
    constructor(storage: StoragePort, audiencePrefix: () => string) {
        super(storage, () => {
            const prefix = audiencePrefix();
            if (!prefix.startsWith("weftext-hosted-draft:"))
                throw invalid();
            return prefix;
        }, mapDraftTexts, prefix => prefix.replace(/^weftext-hosted-draft:/, "weftext-hosted-draft-text:"));
    }
}
const invalid = () => new UiError(uiMessage("hostedDraftCache.recoveryUnreadable"));
function mapDraftTexts(value: unknown, convert: (text: unknown, field: string) => unknown): unknown {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw invalid();
    const draft = value as HostedDraft;
    const identity = (id: unknown) => typeof id === "string" && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id);
    if (draft.inputId !== undefined && !identity(draft.inputId) || draft.viewInputId !== undefined && !identity(draft.viewInputId)
        || draft.basis !== undefined && (!draft.basis || !identity(draft.basis.sessionId) || !identity(draft.basis.baseId))) throw invalid();
    const mapped = { ...draft, source: convert(draft.source, "source"), baseSource: convert(draft.baseSource, "baseSource") };
    if (draft.pending === undefined)
        return mapped;
    if (!draft.pending || typeof draft.pending !== "object" || typeof draft.pending.operationId !== "string" || typeof draft.pending.resource !== "string" || !draft.pending.resource)
        throw invalid();
    const pending = { ...draft.pending, source: convert(draft.pending.source, "pending") };
    if (draft.pending.request)
        pending.request = { ...draft.pending.request, source: convert(draft.pending.request.source, "request") } as typeof draft.pending.request;
    return { ...mapped, pending };
}
