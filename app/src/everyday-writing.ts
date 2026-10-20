import { insertWritingBlock } from "./writing-controls";
import type { WritingController } from "./writing";
import type { EverydayCommand } from "./everyday-navigation";

export function everydayWritingCommands(controller: WritingController | undefined, writable: boolean, insertQuery?: () => Promise<void>): EverydayCommand[] {
  const reason = !controller ? "请先打开笔记。" : !writable ? "当前为只读、待核对或输入组合状态，请先恢复可写会话。" : undefined;
  const actions: EverydayCommand[] = ([ ["paragraph", "普通段落"], ["heading", "章节标题"], ["bulletList", "无序列表"], ["numberedList", "有序列表"], ["image", "图片"] ] as const).map(([kind, label]) => ({
    id: `insert-${kind}`, label: `插入${label}`, detail: "在当前正文末尾添加，共用草稿和撤销", unavailable: reason,
    run: () => insertWritingBlock(controller!, kind),
  }));
  if (insertQuery) actions.push({ id: "insert-query", label: "插入内嵌 Query", detail: "使用当前已安装 schema；确认后进入同一草稿", unavailable: reason, run: insertQuery });
  const selection = controller?.sourceSelection;
  const block = selection && controller?.map?.bodyBlocks?.filter(item => item.range.utf16From <= selection.from && item.range.utf16To >= selection.to).sort((a,b) => a.range.to-a.range.from-(b.range.to-b.range.from))[0];
  const snapshot = controller?.snapshot;
  const checked = async (intent: () => Parameters<WritingController["execute"]>[0]) => {
    await controller?.prepareCommand();
    if (!controller || !snapshot || !controller.isSnapshotCurrent(snapshot)) throw new Error("菜单的正文依据已变化，请重新选择当前块。");
    await controller.execute(intent());
  };
  const selectedBlock = () => { const selected=controller!.sourceSelection; const current=controller!.map?.bodyBlocks?.find(item=>item.range.utf16From<=selected.from&&item.range.utf16To>=selected.to); if(!current)throw new Error("当前选择没有唯一普通根块；原稿保留，请使用准确源码入口。");return current; };
  for (const [target,label] of [["paragraph","普通段落"],["heading","一级标题"],["bulletList","无序列表项"],["numberedList","有序列表项"]]) actions.push({ id: `convert-${target}`, label: `当前块转换为${label}`, unavailable: reason ?? (controller?.map && !block ? "请将光标放在可证明的普通根段落、标题或列表项中；生成、标签与复杂构造保留源码。" : undefined), run: () => checked(() => ({ kind: "convertBodyBlock", range: selectedBlock().range, target, level: 1 })) });
  for (const direction of ["up", "down"] as const) actions.push({ id: `move-${direction}`, label: direction === "up" ? "当前块上移" : "当前块下移", detail: "只移动此块，须有空段分隔的相邻普通根块", unavailable: reason ?? (controller?.map && !block ? "请先选择可证明的普通根块。" : undefined), run: () => checked(() => ({ kind: "moveBodyBlock", range: selectedBlock().range, direction })) });
  actions.push({ id: "undo", label: "撤销", unavailable: reason, run: () => controller!.undo() }, { id: "redo", label: "重做", unavailable: reason, run: () => controller!.redo() });
  return actions;
}
