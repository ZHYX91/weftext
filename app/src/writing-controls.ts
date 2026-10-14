import { rectangularTSV, textareaSourceDraft, type WritingController, type WritingRange, type WritingStructure, type WritingIntent, type WritingBlock, type WritingSnapshot, type TableAction } from "./writing";

export interface WritingControlOptions {
  onError?(error: Error): void;
  insertLink?(): Promise<void>;
  revealSource?(range: WritingRange): void;
}
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: string): HTMLElementTagNameMap[K] { const node = document.createElement(tag); node.className = className; if (text != null) node.textContent = text; return node; }
function report(options: WritingControlOptions, error: unknown): void { options.onError?.(error instanceof Error ? error : new Error(String(error))); }
function action(label: string, run: () => Promise<void> | void, options: WritingControlOptions, className = "writing-button"): HTMLButtonElement {
  const button = element("button", className, label); button.type = "button";
  // Formatting keeps the current source/native text selection when clicked.
  button.addEventListener("mousedown", event => event.preventDefault());
  button.onclick = () => { Promise.resolve().then(run).catch(error => report(options, error)); };
  return button;
}
function field(label: string, input: HTMLElement): HTMLLabelElement { const wrapper = element("label", "writing-field"); wrapper.append(element("span", "", label), input); return wrapper; }
function textInput(value = "", rows = 3): HTMLTextAreaElement { const input = element("textarea", "writing-input"); input.value = value; input.rows = rows; return input; }
function numberInput(value: number, min: number, max?: number): HTMLInputElement { const input = element("input", "writing-input"); input.type = "number"; input.min = String(min); if (max != null) input.max = String(max); input.value = String(value); input.step = "1"; return input; }
function checkInput(label: string, value: boolean): { field: HTMLLabelElement; input: HTMLInputElement } { const input = element("input"); input.type = "checkbox"; input.checked = value; const wrapper = element("label", "writing-checkbox"); wrapper.append(input, document.createTextNode(label)); return { field: wrapper, input }; }
function selectInput(choices: [string, string][], value: string): HTMLSelectElement { const select = element("select", "writing-input"); for (const [key, label] of choices) { const option = element("option", "", label); option.value = key; select.append(option); } select.value = value; return select; }
function validInteger(input: HTMLInputElement): number { const value = Number(input.value); if (!input.checkValidity() || !Number.isSafeInteger(value)) throw new Error("请输入范围内的整数。"); return value; }
function guard(controller: WritingController, snapshot: WritingSnapshot, intent: WritingIntent): () => WritingIntent {
  return () => { if (!controller.isSnapshotCurrent(snapshot)) throw new Error("打开控件后草稿已变化，请重新打开并检查当前内容。"); return intent; };
}

/** Accessible host dialog: source stays in the shared controller until apply. */
async function dialog(title: string, help: string, build: (form: HTMLFormElement, done: () => void, error: HTMLElement) => void): Promise<void> {
  const host = element("dialog", "writing-dialog"); const form = element("form", "writing-form"); const heading = element("h2", "", title);
  const id = `writing-dialog-${Date.now()}-${Math.random().toString(16).slice(2)}`; heading.id = id; host.setAttribute("aria-labelledby", id);
  form.append(heading, element("p", "writing-help", help)); const error = element("p", "writing-error"); error.setAttribute("role", "alert"); error.hidden = true;
  await new Promise<void>(resolve => {
    let settled = false;
    const done = () => { if (settled) return; settled = true; host.close(); host.remove(); resolve(); };
    host.addEventListener("cancel", event => { event.preventDefault(); done(); });
    build(form, done, error); form.append(error); host.append(form); document.body.append(host); host.showModal();
  });
}
function actions(form: HTMLFormElement, done: () => void, label = "应用"): HTMLButtonElement {
  const row = element("div", "writing-actions"); const cancel = element("button", "writing-button", "取消"); cancel.type = "button"; cancel.onclick = done;
  const submit = element("button", "writing-button primary", label); submit.type = "submit"; row.append(cancel, submit); form.append(row); return submit;
}
function submit(form: HTMLFormElement, error: HTMLElement, done: () => void, run: () => Promise<void>): void {
  form.onsubmit = event => {
    event.preventDefault(); if (!form.reportValidity()) return;
    const buttons = Array.from(form.querySelectorAll<HTMLButtonElement>("button")); buttons.forEach(button => { button.disabled = true; });
    Promise.resolve().then(run).then(done).catch(failure => { error.hidden = false; error.textContent = failure instanceof Error ? failure.message : String(failure); buttons.forEach(button => { button.disabled = false; }); });
  };
}

export async function editDocumentRole(controller: WritingController, role: "title" | "subtitle"): Promise<void> {
  await controller.flush(); const snapshot = controller.snapshot, map = controller.map; if (!map) throw new Error("等待最新编辑地图后再编辑标题。");
  const current = map[role];
  if (!["absent", "writable"].includes(current.status)) throw new Error(`${role === "title" ? "标题" : "副标题"}无法无损编辑：${current.status}`);
  if (role === "subtitle" && map.title.status !== "writable") throw new Error("请先创建唯一文档标题，再添加副标题。");
  await dialog(role === "title" ? "文档标题" : "文档副标题", role === "title" ? "标题与章节标题分开。清空标题会一并移除与它关联的副标题。" : "副标题与文档标题关联，采用原生 block，不参与章节编号。", (form, done, error) => {
    const input = textInput(current.text ?? "", 2); form.append(field(role === "title" ? "标题文字（留空表示无标题）" : "副标题文字（留空表示移除）", input));
    actions(form, done); submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: role === "title" ? "setTitle" : "setSubtitle", text: input.value || null })));
  });
}

export async function insertWritingBlock(controller: WritingController, initial = "paragraph"): Promise<void> {
  await controller.flush(); const snapshot = controller.snapshot; const at = controller.insertionPoint();
  await dialog("添加内容", "文字按原生 Typst 普通文字编码。公式使用原生数学表达；表格粘贴预览完整行列后一次应用。", (form, done, error) => {
    const kind = selectInput([["paragraph", "普通段落"], ["heading", "章节标题"], ["bulletList", "无序列表"], ["numberedList", "有序列表"], ["termList", "术语列表"], ["math", "公式"], ["table", "普通表格"], ["raw", "代码块"]], initial);
    const input = textInput("", 5); input.setAttribute("aria-label", "新内容");
    const level = numberInput(1, 1, 1024), levelField = field("标题级别（常用 1–9，也可输入更深级别）", level);
    const display = checkInput("独立公式", true); const language = element("input", "writing-input"); language.placeholder = "python / rust / 留空"; const languageField = field("代码语言", language);
    const preview = element("p", "writing-help"); preview.setAttribute("aria-live", "polite");
    const refresh = () => {
      levelField.hidden = kind.value !== "heading"; display.field.hidden = kind.value !== "math"; languageField.hidden = kind.value !== "raw";
      input.placeholder = kind.value === "table" ? "单元格以 Tab 分隔，每行换行。" : kind.value === "termList" ? "每行：术语<Tab>说明" : ["bulletList", "numberedList"].includes(kind.value) ? "每行是一项，可保留空项。" : kind.value === "math" ? "x^2 + y^2 = z^2" : "写下内容…";
      if (kind.value === "table") { try { const grid = rectangularTSV(input.value); preview.textContent = `${grid.length} 行 × ${grid[0].length} 列；全部单元格一次写入。`; } catch (failure) { preview.textContent = failure instanceof Error ? failure.message : String(failure); } }
      else preview.textContent = "";
    };
    kind.onchange = refresh; input.oninput = refresh;
    form.append(field("内容形式", kind), field("内容", input), levelField, display.field, languageField, preview); refresh(); actions(form, done, "添加");
    submit(form, error, done, async () => {
      let block: WritingBlock;
      if (kind.value === "heading") block = { kind: "heading", text: input.value, level: validInteger(level) };
      else if (kind.value === "bulletList" || kind.value === "numberedList") block = { kind: kind.value, items: input.value.replace(/\r\n/g, "\n").split("\n") };
      else if (kind.value === "termList") { const items = input.value.replace(/\r\n/g, "\n").split("\n").map(row => { const [term, ...description] = row.split("\t"); return { term, description: description.join("\t") }; }); block = { kind: "termList", items }; }
      else if (kind.value === "math") block = { kind: "math", text: input.value, display: display.input.checked };
      else if (kind.value === "table") { const grid = rectangularTSV(input.value); block = { kind: "table", columns: grid[0].length, cells: grid.flat() }; }
      else if (kind.value === "raw") block = { kind: "raw", text: input.value, language: language.value || null, block: true };
      else block = { kind: "paragraph", text: input.value };
      await controller.execute(guard(controller, snapshot, { kind: "insertBlock", at, block }));
    });
  });
}

export async function editHeading(controller: WritingController, heading?: WritingStructure): Promise<void> {
  const originSnapshot = controller.snapshot;
  await controller.flush(); if (heading && !controller.isSnapshotCurrent(originSnapshot)) throw new Error("标题来源在输入回写后已变化，请重新选择当前标题。");
  const current = heading ?? controller.structure("heading"); if (!current) { await insertWritingBlock(controller, "heading"); return; }
  const snapshot = controller.snapshot;
  await dialog("章节标题", "每个标题保留自己的真实级别。接正文只改变这个标题，编号、大纲与引用仍使用原生 heading。", (form, done, error) => {
    const level = numberInput(current.level ?? 1, 1, 1024); const runIn = checkInput("标题与紧随的普通正文同段", current.runIn);
    const operation = selectInput([["level", "修改标题级别"], ["runIn", "修改同段设置"]], "level"); form.append(field("操作", operation), field("级别", level), runIn.field);
    const refresh = () => { level.disabled = operation.value !== "level"; runIn.input.disabled = operation.value !== "runIn"; }; operation.onchange = refresh; refresh();
    actions(form, done); submit(form, error, done, () => controller.execute(guard(controller, snapshot, operation.value === "runIn" ? { kind: "setRunIn", range: current.range, enabled: runIn.input.checked } : { kind: "setHeadingLevel", range: current.range, level: validInteger(level) })));
  });
}

export async function editMath(controller: WritingController, math?: WritingStructure): Promise<void> {
  const originSnapshot = controller.snapshot;
  await controller.flush(); if (math && !controller.isSnapshotCurrent(originSnapshot)) throw new Error("公式来源在输入回写后已变化，请重新选择当前公式。");
  const current = math ?? controller.structure("inlineMath") ?? controller.structure("displayMath"); if (!current) { await insertWritingBlock(controller, "math"); return; }
  const snapshot = controller.snapshot;
  await dialog("编辑公式", "公式保留原生数学源；可视符号作为完整公式呈现，修改后由官方编译器预览。", (form, done, error) => {
    const input = textInput(current.body?.raw ?? current.range.raw.replace(/^\$\s?|\s?\$$/g, ""), 4); input.classList.add("writing-math-source"); input.setAttribute("aria-label", "原生 Typst 数学表达");
    const display = checkInput("独立公式", current.kind === "displayMath"); form.append(field("数学表达", input), display.field); actions(form, done);
    submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: "replaceMath", range: current.range, text: input.value, display: display.input.checked })));
  });
}

export async function editRaw(controller: WritingController, raw?: WritingStructure): Promise<void> {
  const originSnapshot = controller.snapshot;
  await controller.flush(); if (raw && !controller.isSnapshotCurrent(originSnapshot)) throw new Error("代码来源在输入回写后已变化，请重新选择当前代码。");
  const current = raw ?? controller.structure("inlineRaw") ?? controller.structure("blockRaw");
  if (!current) { await insertWritingBlock(controller, "raw"); return; }
  if (!current.body) throw new Error("这段代码没有可证明的原生内容范围，请使用源码入口。");
  const snapshot = controller.snapshot;
  await dialog("编辑代码", "代码内容保留原字节和缩进。语言只选择原生代码高亮，中文注释和字符串无需额外标记。", (form, done, error) => {
    const input = textInput(current.body!.raw, 8); input.classList.add("writing-math-source"); input.spellcheck = false;
    const language = element("input", "writing-input"); language.value = current.language ?? ""; language.placeholder = "python / rust / 留空";
    const block = checkInput("独立代码块", current.kind === "blockRaw"); form.append(field("代码内容", input), field("代码语言", language), block.field); actions(form, done);
    submit(form, error, done, () => controller.execute(guard(controller, snapshot, { kind: "replaceRaw", range: current.range, text: textareaSourceDraft(current.body!.raw, input.value), language: language.value || null, block: block.input.checked })));
  });
}

function decodedCell(controller: WritingController, range: WritingRange): string | null {
  if (!range.raw) return "";
  const candidates = controller.map?.literalRegions.filter(region => region.range.utf16From === range.utf16From && region.range.utf16To === range.utf16To) ?? [];
  return candidates.length === 1 ? candidates[0].text : null;
}
export async function editTable(controller: WritingController, structure?: WritingStructure): Promise<void> {
  const originSnapshot = controller.snapshot;
  await controller.flush(); if (structure && !controller.isSnapshotCurrent(originSnapshot)) throw new Error("表格来源在输入回写后已变化，请重新选择当前表格。");
  const current = structure ?? controller.structure("table"); if (!current?.table) { await insertWritingBlock(controller, "table"); return; }
  const table = current.table, snapshot = controller.snapshot;
  await dialog("表格", "按行列定位准确单元格；相同文字的单元格仍分别处理。复杂拓扑保留原源，网格操作只用于可证明的普通表格。", (form, done, error) => {
    const operation = selectInput([["cell", "编辑单元格文字"], ["insertRow", "插入行"], ["removeRow", "删除行"], ["insertColumn", "插入列"], ["removeColumn", "删除列"], ["moveRow", "移动行"], ["paste", "粘贴矩形 TSV"]], "cell");
    const row = numberInput(1, 1, table.rows), column = numberInput(1, 1, table.columns), destination = numberInput(1, 1, table.rows); const rowField = field("行（从 1 开始）", row), columnField = field("列（从 1 开始）", column), destinationField = field("移到第几行", destination);
    const input = textInput("", 4), inputField = field("单元格文字 / TSV", input); const note = element("p", "writing-help");
    const refresh = () => {
      const selected = operation.value;
      destinationField.hidden = selected !== "moveRow"; inputField.hidden = !["cell", "paste"].includes(selected);
      rowField.hidden = ["insertColumn", "removeColumn"].includes(selected); columnField.hidden = ["insertRow", "removeRow", "moveRow"].includes(selected);
      row.max = String(table.rows + (selected === "insertRow" ? 1 : 0)); column.max = String(table.columns + (selected === "insertColumn" ? 1 : 0));
      if (selected === "cell") {
        const cell = table.cells[(Number(row.value) - 1) * table.columns + Number(column.value) - 1]; const text = cell ? decodedCell(controller, cell) : null;
        input.value = text ?? ""; input.disabled = text == null; note.textContent = text == null ? "此单元格包含结构或生成内容，保留原源；请使用源码入口。" : `${table.rows} 行 × ${table.columns} 列。`;
      } else { input.disabled = false; input.value = ""; note.textContent = table.gridWritable ? "网格变更一次应用，可通过共同撤销栈撤销。" : "此表格有复杂拓扑或不明确的结构，网格变更不可用。"; }
    };
    operation.onchange = refresh; row.onchange = refresh; column.onchange = refresh;
    form.append(field("操作", operation), rowField, columnField, destinationField, inputField, note); refresh(); actions(form, done);
    submit(form, error, done, async () => {
      const selected = operation.value;
      if (selected === "cell") {
        const cell = table.cells[(validInteger(row) - 1) * table.columns + validInteger(column) - 1]; if (!cell || decodedCell(controller, cell) == null) throw new Error("当前单元格不能无损文字编辑。");
        await controller.execute(guard(controller, snapshot, { kind: "setTableCell", range: cell, text: input.value })); return;
      }
      if (!table.gridWritable) throw new Error("此表格的网格操作不可用，原源已保留。");
      let change: TableAction;
      if (selected === "moveRow") change = { kind: "moveRow", from: validInteger(row) - 1, to: validInteger(destination) - 1 };
      else if (selected === "paste") { const grid = rectangularTSV(input.value); const r = validInteger(row) - 1, c = validInteger(column) - 1; if (r + grid.length > table.rows || c + grid[0].length > table.columns) throw new Error(`粘贴区域为 ${grid.length} 行 × ${grid[0].length} 列，超出当前表格；请先插入行列。`); change = { kind: "paste", row: r, column: c, text: input.value }; }
      else change = { kind: selected as "insertRow" | "removeRow" | "insertColumn" | "removeColumn", index: validInteger(selected.includes("Column") ? column : row) - 1 };
      await controller.execute(guard(controller, snapshot, { kind: "tableEdit", range: current.range, action: change }));
    });
  });
}

export function createWritingToolbar(controller: WritingController, options: WritingControlOptions = {}): HTMLElement {
  const toolbar = element("div", "writing-toolbar"); toolbar.setAttribute("role", "toolbar"); toolbar.setAttribute("aria-label", "写作工具");
  const tool = (label: string, run: () => Promise<void>) => action(label, () => controller.deferUntilCompositionEnds(run), options);
  for (const [label, format] of [["粗体", "strong"], ["斜体", "emph"], ["删除线", "strike"]] as const) toolbar.append(tool(label, () => controller.execute(() => ({ kind: "setInline", range: controller.selectionRange(), format }))));
  toolbar.append(tool("标题", () => editDocumentRole(controller, "title")), tool("副标题", () => editDocumentRole(controller, "subtitle")), tool("章节", () => editHeading(controller)), tool("列表", () => insertWritingBlock(controller, "bulletList")), tool("表格", () => editTable(controller)), tool("公式", () => editMath(controller)), tool("代码", () => editRaw(controller)));
  if (options.insertLink) toolbar.append(tool("链接", options.insertLink));
  toolbar.append(tool("添加内容", () => insertWritingBlock(controller)), tool("撤销", () => controller.undo()), tool("重做", () => controller.redo()));
  return toolbar;
}

export function createStructureControls(controller: WritingController, range: WritingRange, options: WritingControlOptions = {}): HTMLElement | null {
  const structure = controller.map?.structures.find(item => item.range.utf16From === range.utf16From && item.range.utf16To === range.utf16To && item.range.raw === range.raw);
  if (!structure) return null;
  const snapshot = controller.snapshot;
  const checked = (run: () => Promise<void> | void) => async () => { if (!controller.isSnapshotCurrent(snapshot)) throw new Error("这个结构入口已过期，请等待当前预览后重试。"); await run(); };
  const controls = element("div", "writing-structure-controls"); controls.setAttribute("role", "group"); controls.setAttribute("aria-label", "当前结构操作");
  if (structure.kind === "heading") controls.append(action(`章节 ${structure.level} · ${structure.runIn ? "接正文" : "独立段"}`, checked(() => editHeading(controller, structure)), options));
  else if (structure.kind === "table") controls.append(action("编辑表格", checked(() => editTable(controller, structure)), options));
  else if (["inlineMath", "displayMath"].includes(structure.kind)) controls.append(action("编辑公式", checked(() => editMath(controller, structure)), options));
  else if (["inlineRaw", "blockRaw"].includes(structure.kind)) controls.append(action("编辑代码", checked(() => editRaw(controller, structure)), options));
  else if (["bulletListItem", "numberedListItem"].includes(structure.kind)) {
    const execute = (kind: "indent" | "outdent") => controller.execute(guard(controller, snapshot, { kind: "listEdit", range: structure.range, action: { kind } }));
    controls.append(action("缩进", checked(() => execute("indent")), options), action("减少缩进", checked(() => execute("outdent")), options));
  }
  if (options.revealSource) controls.append(action("编辑来源", checked(() => options.revealSource!(structure.range)), options));
  return controls.childElementCount ? controls : null;
}
