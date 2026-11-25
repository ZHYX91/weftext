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
    let generation = 0, disposed = false, busy = false;
    let rowScope: LocaleScope | undefined;
    let choices: HTMLButtonElement[] = [];
    const owner = {
        assertCloseReady: () => guard.assertReady(false),
        freezeForClose: () => guard.freeze(false),
        thawAfterClose: () => guard.thaw(),
    };
    const unregister = registerNativeProposalOwner(owner);
    const current = (cut: number): boolean => !disposed && !guard.frozen && generation === cut && dialog.isConnected;
    const refresh = document.createElement("button"), close = document.createElement("button");
    for (const button of [refresh, close]) { button.type = "button"; button.className = "data-button"; }
    scope.bindText(refresh, message("reminder.refresh")); scope.bindText(close, message("common.close"));
    const updateAvailability = (): void => {
        // aria-disabled keeps the focused control in place while admission is suspended.
        for (const button of [close, refresh, ...choices]) {
            if (busy) button.setAttribute("aria-disabled", "true");
            else button.removeAttribute("aria-disabled");
        }
    };
    const run = (cut: number, operation: () => Promise<boolean>): void => {
        if (busy || !current(cut)) return;
        busy = true;
        updateAvailability();
        void guard.run(operation).then(adopted => {
            busy = false;
            updateAvailability();
            if (adopted && current(cut)) dismiss();
        }, error => {
            busy = false;
            updateAvailability();
            if (current(cut)) host.notify(errorUiText(error), true);
        });
    };
    const render = (activation: ReminderActivation): void => {
        const cut = ++generation;
        rowScope?.dispose();
        const labels = rowScope = new LocaleScope();
        choices = [];
        body.replaceChildren();
        const heading = document.createElement("h2"); labels.bindText(heading, message("reminder.activationTitle")); body.append(heading);
        if (activation.issues.length) { const text = document.createElement("p"); labels.bindText(text, message("reminder.activationLimited")); body.append(text); }
        if (!activation.candidates.length) { const text = document.createElement("p"); labels.bindText(text, message("reminder.activationEmpty")); body.append(text); }
        activation.candidates.forEach((member, index) => {
            const row = document.createElement("section"), label = document.createElement("p"), open = document.createElement("button");
            label.textContent = [member.root, member.resolution.triggerInstant].filter(Boolean).join(" · ");
            open.type = "button"; open.className = "data-button"; labels.bindText(open, message("reminder.activationTask", { p0: index + 1 }));
            open.onclick = () => run(cut, async () => {
                await openReminderActivationMember(host, activationKey, member.owner, () => current(cut));
                return true;
            });
            choices.push(open); row.append(label, open); body.append(row);
        });
        updateAvailability();
    };
    refresh.onclick = () => {
        const cut = generation;
        run(cut, async () => {
            const fresh = decode(await host.request("/api/reminders/activation", { activationKey }));
            if (current(cut)) render(fresh);
            return false;
        });
    };
    const dismiss = (): void => {
        if (busy) return;
        try { guard.assertMutable(); guard.assertReady(false); }
        catch (error) { host.notify(errorUiText(error), true); return; }
        dialog.close();
    };
    close.onclick = dismiss;
    dialog.addEventListener("cancel", event => { event.preventDefault(); dismiss(); });
    dialog.addEventListener("close", () => {
        try { guard.assertMutable(); guard.assertReady(false); }
        catch (error) { host.notify(errorUiText(error), true); dialog.showModal(); return; }
        disposed = true; generation++; unregister(); rowScope?.dispose(); scope.dispose(); dialog.remove();
    });
    render(original); dialog.append(close, refresh, body); document.body.append(dialog);
    try { dialog.showModal(); }
    catch (error) { disposed = true; unregister(); rowScope?.dispose(); scope.dispose(); dialog.remove(); throw error; }
}
