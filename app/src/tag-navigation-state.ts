import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Recovery inspects the original operation; it never substitutes a fresh plan. */
export function tagOperationStatus(expected: {
    operationId: string;
    planId: string;
}, incoming: {
    operationId?: string;
    planId?: string;
    status?: string;
    receipt?: {
        status?: string;
    };
}): "prepared" | "committed" {
    if (incoming.operationId !== expected.operationId || incoming.planId !== expected.planId)
        throw new UiError(uiMessage("tagNavigationState.recoveryMismatch"));
    const status = incoming.receipt?.status ?? incoming.status;
    if (status !== "prepared" && status !== "committed")
        throw new UiError(displayMessage("tagNavigationState.originalUnconfirmed", { p0: uiDisplayParameter(status ?? uiMessage("common.unknown")) }));
    return status;
}
export function tagCancellationStatus(expected: {
    operationId: string;
    planId: string | null;
}, incoming: {
    operationId: string;
    planId: string | null;
    status: string;
}): "resolved" | "committed" {
    if (incoming.operationId !== expected.operationId || expected.planId !== null && incoming.planId !== expected.planId)
        throw new UiError(uiMessage("tagNavigationState.cancellationReceiptMismatch"));
    if (incoming.planId !== null && (typeof incoming.planId !== "string" || !incoming.planId))
        throw new UiError(uiMessage("tagNavigationState.cancellationPlanStateMissing"));
    if (incoming.status === "cancelled" || incoming.status === "resolved")
        return "resolved";
    if (incoming.status === "committed" && typeof incoming.planId === "string")
        return "committed";
    throw new UiError(displayMessage("tagNavigationState.cancellationUnconfirmed", { p0: uiDisplayParameter(incoming.status) }));
}
