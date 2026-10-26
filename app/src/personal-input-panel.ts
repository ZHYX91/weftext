import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { PersonalInputRecovery, personalInputText, type PersonalInputRecord } from "./personal-input-recovery";
import type { AuthorProducerState } from "./author-input";
export function renderPersonalInputs(host: HTMLElement, recovery: PersonalInputRecovery, audience: string, actions: {
    restore(record: PersonalInputRecord): Promise<void>;
    operation(record: PersonalInputRecord): Promise<void>;
    producers(): AuthorProducerState[];
    discardProducer(producer: AuthorProducerState): Promise<void>;
    changed(): void;
}): void {
    host.replaceChildren();
    const title = document.createElement("h2");
    setUiText(title, uiMessage("personalInput.title"));
    host.append(title);
    const help = document.createElement("p");
    setUiText(help, uiMessage("personalInput.help"));
    host.append(help);
    const notice = recovery.notice(audience);
    if (notice) {
        const warning = document.createElement("p");
        setUiText(warning, notice);
        warning.setAttribute("role", "alert");
        host.append(warning);
    }
    for (const record of recovery.list(audience)) {
        const row = document.createElement("section"), input = document.createElement("textarea"), status = document.createElement("p");
        const unknownOriginalRoute = record.pending?.route === "unknownOriginalRoute";
        const unresolvedOriginalRoute = unknownOriginalRoute && record.pending?.status === "unresolved";
        input.value = personalInputText(record);
        input.readOnly = true;
        input.rows = 5;
        setUiAttr(input, "aria-label", uiMessage("personalInput.textLabel"));
        status.setAttribute("role", "status");
        const action = (label: UiText, work: () => void | Promise<void>) => {
            const button = document.createElement("button");
            button.type = "button";
            setUiText(button, label);
            button.onclick = () => { button.disabled = true; Promise.resolve().then(work).then(() => { actions.changed(); }).catch(error => { setUiText(status, error instanceof Error ? errorUiText(error) : String(error)); }).finally(() => { button.disabled = false; }); };
            return button;
        };
        row.append(input, action(uiMessage("personalInput.copyText"), async () => {
            input.focus();
            input.select();
            if (navigator.clipboard?.writeText)
                await navigator.clipboard.writeText(input.value);
            setUiText(status, uiMessage("personalInput.textRetrieved"));
        }));
        if (record.pending) {
            const operation = document.createElement("p");
            setUiText(operation, displayMessage("personalInput.originalSaveSummary", { p0: uiDisplayParameter(record.pending.operationId), p1: uiDisplayParameter(record.pending.status) }));
            row.append(operation);
        }
        if (unresolvedOriginalRoute) {
            const reason = document.createElement("p");
            setUiText(reason, uiMessage("personalInput.unknownOriginalRoute"));
            row.append(reason);
        }
        if (record.pending?.status === "unresolved") {
            const resolve = action(uiMessage("personalInput.resolveOriginalSave"), () => actions.operation(record));
            resolve.disabled = unknownOriginalRoute;
            row.append(resolve);
        }
        if (record.owner.kind === "body") {
            const restore = action(uiMessage("personalInput.restoreDraft"), () => actions.restore(record));
            restore.disabled = unresolvedOriginalRoute;
            row.append(restore);
        }
        const remove = action(uiMessage("personalInput.removeRetrieved"), async () => { await recovery.remove(audience, record); row.remove(); });
        remove.disabled = record.pending?.status === "unresolved";
        row.append(remove, status);
        host.append(row);
    }
    for (const producer of actions.producers().filter(item => item.retired && (item.unapplied || item.nativePending))) {
        const row = document.createElement("section"), status = document.createElement("p"), description = document.createElement("p"), discard = document.createElement("button");
        description.textContent = `已退休表单 ${producer.owner.kind} · ${producer.producerId}：其输入尚未应用，仍会阻止关闭。移除单条恢复副本不会放弃整个表单。`;
        discard.type = "button";
        discard.textContent = "明确放弃此退休表单的全部未应用输入";
        discard.disabled = recovery.list(audience).some(record => record.producer?.producerId === producer.producerId && record.pending?.status === "unresolved");
        discard.onclick = () => { discard.disabled = true; void actions.discardProducer(producer).then(actions.changed).catch(error => { status.textContent = error instanceof Error ? error.message : String(error); discard.disabled = false; }); };
        status.setAttribute("role", "status");
        row.append(description, discard, status);
        host.append(row);
    }
}
