import type { PanelHost } from "./data-panel";
import type { OpenedDocument } from "./types";
import type { ReminderOwner } from "./reminder-types";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { registerNativeProposalOwner } from "./native-proposal-owners";
import { LocaleScope, errorUiText, message, UiError } from "./ui-locale";

interface Member { root: string; owner: ReminderOwner; resolution: { triggerInstant: string | null } }
export interface ReminderActivation { candidates: Member[]; issues: unknown[] }
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
function decode(value: unknown): ReminderActivation {
    const result = value as ReminderActivation | null;
    if (!result || !Array.isArray(result.candidates) || result.candidates.length > 64 || !Array.isArray(result.issues)
        || result.candidates.some(member => !member || typeof member.root !== "string" || !member.owner
            || !uuid.test(member.owner.workspaceId) || !uuid.test(member.owner.nodeId) || !member.resolution
            || (member.resolution.triggerInstant !== null && typeof member.resolution.triggerInstant !== "string")))
        throw new UiError(message("error.invalidResponse"));
    return result;
}

/** Re-authorize the original notification member immediately before opening.
 * A late, unadopted OpenResult is released; it cannot become another document. */
export async function openReminderActivationMember(host: PanelHost, activationKey: string, owner: ReminderOwner, current = () => true): Promise<void> {
    if (!host.openResolvedConfirmed || !current()) throw new UiError(message("shell.closeBasisChanged"));
    await host.prepareAction?.();
    if (!current()) throw new UiError(message("shell.closeBasisChanged"));
    const document = await host.request<OpenedDocument>("/api/reminders/activation/open", { activationKey, owner });
    if (!current()) {
        await host.releaseOpenedSession?.(document.sessionId);
        throw new UiError(message("shell.closeBasisChanged"));
    }
    if (!await host.openResolvedConfirmed(document))
        throw new UiError(message("shell.closeBasisChanged"));
}

/** One member navigates only after the real owner accepts it. A summary,
 * expired notification or restricted member set is presented explicitly. */
export async function activateReminderNotification(host: PanelHost, activationKey: string): Promise<void> {
    if (!uuid.test(activationKey)) throw new UiError(message("error.invalidResponse"));
    const original = decode(await host.request("/api/reminders/activation", { activationKey }));
    if (original.candidates.length === 1 && original.issues.length === 0) {
        await openReminderActivationMember(host, activationKey, original.candidates[0].owner);
        return;
    }
    const dialog = document.createElement("dialog"), body = document.createElement("section"), scope = new LocaleScope(), guard = new ProposalCloseGuard();
    dialog.className = "capture-dialog reminder-activation-dialog";
    scope.bindAttr(dialog, "aria-label", message("reminder.activationTitle"));
    guard.bind(dialog);
    let generation = 0, disposed = false;
    const owner = {
        assertCloseReady: () => guard.assertReady(false),
        freezeForClose: () => guard.freeze(false),
        thawAfterClose: () => guard.thaw(),
    };
    const unregister = registerNativeProposalOwner(owner);
    const current = (cut: number): boolean => !disposed && !guard.frozen && generation === cut && dialog.isConnected;
    const render = (activation: ReminderActivation): void => {
        const cut = ++generation;
        body.replaceChildren();
        const heading = document.createElement("h2"); scope.bindText(heading, message("reminder.activationTitle")); body.append(heading);
        if (activation.issues.length) { const text = document.createElement("p"); scope.bindText(text, message("reminder.activationLimited")); body.append(text); }
        if (!activation.candidates.length) { const text = document.createElement("p"); scope.bindText(text, message("reminder.activationEmpty")); body.append(text); }
        activation.candidates.forEach((member, index) => {
            const row = document.createElement("section"), label = document.createElement("p"), open = document.createElement("button");
            label.textContent = [member.root, member.resolution.triggerInstant].filter(Boolean).join(" · ");
            open.type = "button"; open.className = "data-button"; scope.bindText(open, message("reminder.activationTask", { p0: index + 1 }));
            open.onclick = () => {
                if (!current(cut) || open.disabled) return;
                open.disabled = true;
                let adopted = false;
                void guard.run(async () => {
                    try { await openReminderActivationMember(host, activationKey, member.owner, () => current(cut)); adopted = true; }
                    catch (error) { if (current(cut)) host.notify(errorUiText(error), true); }
                    finally { if (current(cut)) open.disabled = false; }
                }).then(() => { if (adopted && current(cut)) dismiss(); }).catch(error => host.notify(errorUiText(error), true));
            };
            row.append(label, open); body.append(row);
        });
    };
    const refresh = document.createElement("button"), close = document.createElement("button");
    for (const button of [refresh, close]) { button.type = "button"; button.className = "data-button"; }
    scope.bindText(refresh, message("reminder.refresh")); scope.bindText(close, message("common.close"));
    refresh.onclick = () => {
        if (refresh.disabled || disposed) return;
        refresh.disabled = true;
        void guard.run(async () => {
            try { const fresh = decode(await host.request("/api/reminders/activation", { activationKey })); if (!disposed) render(fresh); }
            catch (error) { if (!disposed) host.notify(errorUiText(error), true); }
            finally { if (!disposed) refresh.disabled = false; }
        }).catch(error => host.notify(errorUiText(error), true));
    };
    const dismiss = (): void => {
        try { guard.assertMutable(); guard.assertReady(false); }
        catch (error) { host.notify(errorUiText(error), true); return; }
        dialog.close();
    };
    close.onclick = dismiss;
    dialog.addEventListener("cancel", event => { event.preventDefault(); dismiss(); });
    dialog.addEventListener("close", () => {
        try { guard.assertMutable(); guard.assertReady(false); }
        catch (error) { host.notify(errorUiText(error), true); dialog.showModal(); return; }
        disposed = true; generation++; unregister(); scope.dispose(); dialog.remove();
    });
    render(original); dialog.append(close, refresh, body); document.body.append(dialog);
    try { dialog.showModal(); }
    catch (error) { disposed = true; unregister(); scope.dispose(); dialog.remove(); throw error; }
}
