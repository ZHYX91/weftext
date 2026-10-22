import { PersonalInputRecovery, personalInputText, type PersonalInputRecord } from "./personal-input-recovery";
import type { AuthorProducerState } from "./author-input";

export function renderPersonalInputs(host: HTMLElement, recovery: PersonalInputRecovery, audience: string, actions: { restore(record: PersonalInputRecord): Promise<void>; operation(record: PersonalInputRecord): Promise<void>; producers(): AuthorProducerState[]; discardProducer(producer: AuthorProducerState): Promise<void>; changed(): void }): void {
  host.replaceChildren(); const title = document.createElement("h2"); title.textContent = "本人未上传输入与原操作"; host.append(title);
  const help = document.createElement("p"); help.textContent = "这里只保存本人新增文字、原基线位置与原操作编号，不含旧正文、预览或凭据。先打开获权的原笔记；仅在原身份与准确基线一致时可恢复为草稿。表单等无法映射的文字只供单独取回，可能已经应用或撤销，不代表当前正文。"; host.append(help);
  const notice = recovery.notice(audience); if (notice) { const warning = document.createElement("p"); warning.textContent = notice; warning.setAttribute("role", "alert"); host.append(warning); }
  for (const record of recovery.list(audience)) {
    const row = document.createElement("section"), input = document.createElement("textarea"), status = document.createElement("p");
    input.value = personalInputText(record); input.readOnly = true; input.rows = 5; input.setAttribute("aria-label", "本人新增输入"); status.setAttribute("role", "status");
    const action = (label: string, work: () => void | Promise<void>) => {
      const button = document.createElement("button"); button.type = "button"; button.textContent = label;
      button.onclick = () => { button.disabled = true; Promise.resolve().then(work).then(() => { actions.changed(); }).catch(error => { status.textContent = error instanceof Error ? error.message : String(error); }).finally(() => { button.disabled = false; }); };
      return button;
    };
    row.append(input, action("复制本人新增文字", async () => { input.focus(); input.select(); if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(input.value); status.textContent = "文字已选中或复制；原恢复记录继续保留。"; }));
    if (record.pending) { const operation = document.createElement("p"); operation.textContent = `原保存操作 ${record.pending.operationId} · ${record.pending.status}`; row.append(operation); }
    if (record.pending?.status === "unresolved") row.append(action("查询并核定原保存操作", () => actions.operation(record)));
    if (record.owner.kind === "body") row.append(action("核对原基线并恢复为草稿", () => actions.restore(record)));
    const remove = action("已取回，移除此份", async () => { await recovery.remove(audience, record); row.remove(); }); remove.disabled = record.pending?.status === "unresolved";
    row.append(remove, status); host.append(row);
  }
  for (const producer of actions.producers().filter(item => item.retired && (item.unapplied || item.nativePending))) {
    const row = document.createElement("section"), status = document.createElement("p"), description = document.createElement("p"), discard = document.createElement("button");
    description.textContent = `已退休表单 ${producer.owner.kind} · ${producer.producerId}：其输入尚未应用，仍会阻止关闭。移除单条恢复副本不会放弃整个表单。`;
    discard.type = "button"; discard.textContent = "明确放弃此退休表单的全部未应用输入";
    discard.disabled = recovery.list(audience).some(record => record.producer?.producerId === producer.producerId && record.pending?.status === "unresolved");
    discard.onclick = () => { discard.disabled = true; void actions.discardProducer(producer).then(actions.changed).catch(error => { status.textContent = error instanceof Error ? error.message : String(error); discard.disabled = false; }); };
    status.setAttribute("role", "status"); row.append(description, discard, status); host.append(row);
  }
}
