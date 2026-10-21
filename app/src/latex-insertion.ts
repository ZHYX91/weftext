import { createLatexMathPanel, type LatexMathPanel } from "./math-latex";
import { nativeElement } from "./native-node";
import { sourceBoundary } from "./editor";
import type { PanelHost } from "./data-panel";
import type { RenderNode } from "./types";

type Host = PanelHost & { selection?(): { from: number; to: number } | undefined; editable?(): boolean };
interface Preview { ok: boolean; tree: RenderNode | null; source: string; serial: number; message?: string }

/** Conversion uses a frozen insertion basis. Preview is a separate consumer and
 * can never register the proposed source as the editor's current version. */
export class LatexInsertionPanel {
  private dialogs = new Map<string, { dialog: HTMLDialogElement; panel: LatexMathPanel; source: string }>();
  constructor(private host: Host) {}
  clear(): void {
    for (const item of this.dialogs.values()) { item.panel.destroy(); item.dialog.close(); item.dialog.remove(); }
    this.dialogs.clear();
  }
  async open(): Promise<void> {
    await this.host.prepareAction?.();
    const basis = this.host.getDocument();
    if (!basis || this.host.editable?.() === false) throw new Error("请先打开可编辑笔记，再插入公式。");
    const selection = this.host.selection?.() ?? { from: basis.source.length, to: basis.source.length };
    if (!sourceBoundary(basis.source, selection.from) || !sourceBoundary(basis.source, selection.to) || selection.to < selection.from) throw new Error("插入选区不属于当前完整字符，原稿保留。");
    const key = `${basis.sessionId}:${basis.baseId}:${basis.serial}:${selection.from}:${selection.to}`, retained = this.dialogs.get(key);
    if (retained && retained.source === basis.source) { if (!retained.dialog.open) retained.dialog.showModal(); return; }
    const current = (): boolean => {
      const value = this.host.getDocument();
      return !!value && value.sessionId === basis.sessionId && value.source === basis.source && value.serial === basis.serial && value.baseId === basis.baseId && this.host.editable?.() !== false;
    };
    const proposed = (text: string): string => basis.source.slice(0, selection.from) + text + basis.source.slice(selection.to);
    const dialog = document.createElement("dialog"); dialog.className = "modal writing-dialog latex-insertion";
    const close = document.createElement("button"); close.type = "button"; close.textContent = "关闭";
    const panel = createLatexMathPanel({
      authorInputs: this.host.authorInputs?.(basis),
      preview: async result => {
        if (!current()) throw new Error("原笔记或选区依据已变化，请保留输入并重新选择插入位置。");
        const candidate = proposed(result.typst);
        const resultView = await this.host.request<Preview>("/api/editor/preview", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, proposalSource: candidate });
        if (!current() || resultView.source !== candidate || resultView.serial !== basis.serial) throw new Error("预览期间草稿已变化，未采用旧结果。");
        if (!resultView.ok || !resultView.tree) throw new Error(resultView.message || "该位置的原生公式未编译成功，请检查插入位置和原文。");
        const view = document.createElement("div"); view.className = "native-proposal-preview";
        view.setAttribute("aria-label", "插入后的完整原生文档预览");
        const draw = (node: RenderNode): Node => {
          if (node.kind === "text") return document.createTextNode(node.text ?? "");
          if (node.kind === "tag") return document.createDocumentFragment();
          if (node.kind === "frame") { const hint = document.createElement("p"); hint.textContent = "此原生布局需分页查看。"; return hint; }
          const element = nativeElement(node);
          for (const child of node.children) element.append(draw(child));
          if (node.text) element.append(document.createTextNode(node.text));
          return element;
        };
        view.append(draw(resultView.tree)); return view;
      },
      apply: async result => {
        if (!current()) throw new Error("笔记或编辑资格已变化，未替换新草稿。");
        await this.host.applyPatch({ from: selection.from, to: selection.to, text: result.typst }, basis);
        this.host.notify("公式已进入当前草稿，可撤销；保存沿用当前笔记。");
      },
    });
    this.dialogs.set(key, { dialog, panel, source: basis.source });
    close.onclick = () => dialog.close();
    dialog.addEventListener("cancel", event => { event.preventDefault(); dialog.close(); });
    const position = document.createElement("p"); position.textContent = selection.from === selection.to ? "预览并确认后在所选位置插入；原稿尚未改动。" : "预览并确认后替换所选内容；原稿尚未改动。";
    dialog.append(close, position, panel.element); document.body.append(dialog); dialog.showModal();
  }
}
