import { action, ui, type DocumentBasis, type PanelHost } from "./data-panel";
import { DiagramController, diagramOccurrences, diagramPatch, type DiagramOccurrence } from "./diagram-state";
import { renderDiagramPng } from "./diagram-render";
import "./diagram.css";

/** Host supplies the same current-draft and permission checked patch route. */
export class DiagramPanel {
  private readonly dialogs = new Map<string, { dialog: HTMLDialogElement; clear(): void }>();
  constructor(private readonly host: PanelHost, private readonly editable: () => boolean = () => true) {}
  clear(): void { for (const item of this.dialogs.values()) item.clear(); this.dialogs.clear(); }
  async open(): Promise<void> {
    if (!this.editable()) { this.host.notify("当前笔记不可编辑，不能修改图源。", true); return; }
    await this.host.prepareAction?.(); const basis = this.host.getDocument();
    if (!basis || !this.editable()) { this.host.notify("请先打开可编辑笔记。", true); return; }
    const key = `${basis.sessionId}/${basis.serial}/${basis.baseId}`; const retained = this.dialogs.get(key);
    if (retained) { if (!retained.dialog.open) retained.dialog.showModal(); return; }
    const item = this.create(basis); this.dialogs.set(key, item); document.body.append(item.dialog); item.dialog.showModal();
  }
  private create(basis: DocumentBasis): { dialog: HTMLDialogElement; clear(): void } {
    let disposed = false; const authorInputs = this.host.authorInputs?.(basis);
    const dialog = ui("dialog", "modal diagram-panel"); const title = ui("h2", "", "Mermaid 图表"); title.id = `diagram-${crypto.randomUUID()}`; dialog.setAttribute("aria-labelledby", title.id);
    const close = action("关闭并保留输入", () => { controller.cancel(); draw(); dialog.close(); }, "data-button compact"); const header = ui("header", "diagram-header"); header.append(title, close);
    let source = ui("textarea", "data-input diagram-source"); source.rows = 10; source.spellcheck = false; source.setAttribute("aria-label", "Mermaid 源码");
    let width = ui("input", "data-input"); width.type = "number"; width.min = "320"; width.max = "2048"; width.step = "1"; width.value = "1200"; width.setAttribute("aria-label", "PNG 图片宽度（像素）");
    const choose = ui("select", "data-input"); choose.setAttribute("aria-label", "图表位置"); const occurrences = diagramOccurrences(basis.source);
    const fresh = ui("option", "", "新建图表（追加到正文末尾）"); fresh.value = "new"; choose.append(fresh);
    occurrences.forEach((item, index) => { const option = ui("option", "", `图表 ${index + 1} · ${item.source.split("\n")[0]}${item.stale ? " · 源已改变，需重生成" : ""}`); option.value = String(index); choose.append(option); });
    let selected: DiagramOccurrence | undefined; let selectionKey = "new"; const inputs = new Map<string,{source:string;width:string}>(); const controller = new DiagramController(renderDiagramPng); source.value = controller.source;
    const preview = ui("div", "diagram-preview"); const status = ui("p", "data-help"); status.setAttribute("role", "status"); status.setAttribute("aria-live", "polite"); let imageUrl: string | null = null;
    const generate = action("生成 PNG 预览", async () => {
      status.className = "data-help"; status.textContent = "正在本机浏览器离线生成；没有写入笔记。";
      const pending = controller.render(); draw();
      try { if (await pending) status.textContent = `已生成当前图源的 ${controller.image!.width} × ${controller.image!.height} PNG。请核对中文与尺寸后确认。`; }
      catch (error) { status.className = "data-warning"; status.textContent = error instanceof Error ? error.message : String(error); }
      finally { draw(); }
    }, "data-button");
    const commit = action("确认当前图片并写入草稿", async () => {
      if (!controller.image || disposed) return; commit.disabled = true;
      try {
        const current = this.host.getDocument();
        if (!this.editable() || !current || current.sessionId !== basis.sessionId || current.serial !== basis.serial || current.baseId !== basis.baseId || current.source !== basis.source) throw new Error("笔记已改变或权限已更新；图源输入仍保留，请重新打开当前笔记的图表入口。");
        const patch = diagramPatch(basis, controller.image, selected); await this.host.applyPatch(patch, basis); if (disposed) return; controller.cancel(); draw(); dialog.close(); this.host.notify("图源与准确 PNG 已作为一次草稿变更写入，可撤销；保存沿用当前笔记。");
      } catch (error) { if (disposed) return; status.className = "data-warning"; status.textContent = error instanceof Error ? error.message : String(error); draw(); }
    }, "data-button primary");
    function draw(): void {
      if (disposed) return;
      generate.disabled = controller.busy; commit.disabled = controller.busy || !controller.image; preview.replaceChildren();
      if (imageUrl) { URL.revokeObjectURL(imageUrl); imageUrl = null; }
      if (controller.image) { imageUrl = URL.createObjectURL(new Blob([new Uint8Array(controller.image.png)], { type: "image/png" })); const image = ui("img"); image.src = imageUrl; image.alt = "当前 Mermaid 图源的 PNG 预览"; preview.append(image); }
    }
    const changed = (): void => { controller.edit(source.value, Number(width.value)); draw(); status.textContent = "图源或尺寸已改变，旧图已失效；请重新生成。"; };
    authorInputs?.bind(source, `diagram:${selectionKey}:source`); authorInputs?.bind(width, `diagram:${selectionKey}:width`); source.oninput = changed; width.oninput = changed;
    choose.onchange = () => { inputs.set(selectionKey,{source:source.value,width:width.value}); authorInputs?.invalidate(`diagram:${selectionKey}:source`); authorInputs?.invalidate(`diagram:${selectionKey}:width`); const replacement = source.cloneNode(false) as HTMLTextAreaElement; source.replaceWith(replacement); source = replacement; source.oninput = changed; const newWidth = width.cloneNode(false) as HTMLInputElement; width.replaceWith(newWidth); width = newWidth; width.oninput = changed; selectionKey=choose.value; selected = selectionKey === "new" ? undefined : occurrences[Number(selectionKey)]; const retained=inputs.get(selectionKey); source.value = retained?.source ?? selected?.source ?? "flowchart LR\n  A[开始] --> B[完成]"; width.value=retained?.width??"1200"; authorInputs?.bind(source, `diagram:${selectionKey}:source`); authorInputs?.bind(width, `diagram:${selectionKey}:width`); changed(); };
    const controls = ui("div", "diagram-actions"); controls.append(generate, commit);
    dialog.append(header, ui("p", "data-help", "图源只有一份可编辑原文。PNG 在当前浏览器离线生成，确认后与原生源一同保存、同步和输出。直接修改源码中的 source 后须重新生成，旧图片不能输出。DOCX 再导入为普通图片。"), choose, source, ui("label", "data-label", "PNG 图片宽度（320–2048 像素）"), width, preview, status, controls);
    dialog.addEventListener("cancel", event => { event.preventDefault(); controller.cancel(); draw(); dialog.close(); });
    return { dialog, clear: () => { disposed = true; authorInputs?.retire(); controller.edit(""); controller.cancel(); source.value = ""; status.textContent = ""; preview.replaceChildren(); choose.replaceChildren(); occurrences.length = 0; inputs.clear(); selected = undefined; if (imageUrl) URL.revokeObjectURL(imageUrl); imageUrl = null; dialog.close(); dialog.remove(); } };
  }
}
