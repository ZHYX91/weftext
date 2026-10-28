import { LocaleScope, message, type UiText } from "./ui-locale";

export type RibbonTab = "file" | "home" | "insert" | "knowledge" | "review" | "view";
const tabs: readonly RibbonTab[] = ["file", "home", "insert", "knowledge", "review", "view"];
let nextRibbonId = 0;

interface Group {
  element: HTMLElement;
  content: HTMLElement;
  label: HTMLElement;
  scope: LocaleScope;
  unbindLabel: () => void;
}
interface Tab {
  button: HTMLButtonElement;
  panel: HTMLElement;
  groups: Map<string, Group>;
}

function knownTab(value: unknown): asserts value is RibbonTab {
  if (!tabs.includes(value as RibbonTab)) throw new Error("Unknown workspace ribbon tab");
}

/** Presentation only. Callers retain every command, author input and proposal
 * owner. Selecting a tab changes visibility, never the lifetime of its nodes.
 * Import workspace-chrome.css once from the local/hosted entry point. */
export class WorkspaceRibbon {
  readonly element = document.createElement("nav");
  private readonly scope = new LocaleScope();
  private readonly tablist = document.createElement("div");
  private readonly toggle = document.createElement("button");
  private readonly entries = new Map<RibbonTab, Tab>();
  private current: RibbonTab = "home";
  private collapsed = false;
  private disposed = false;
  private nextGroupId = 0;

  constructor() {
    const prefix = `weftext-ribbon-${++nextRibbonId}`;
    this.element.className = "workspace-ribbon";
    this.scope.bindAttr(this.element, "aria-label", message("workspaceRibbon.label"));
    this.tablist.className = "workspace-ribbon-tabs";
    this.tablist.setAttribute("role", "tablist");
    this.tablist.setAttribute("aria-orientation", "horizontal");
    this.scope.bindAttr(this.tablist, "aria-label", message("workspaceRibbon.label"));
    this.toggle.type = "button";
    this.toggle.className = "workspace-ribbon-toggle";
    this.scope.bindText(this.toggle, () => message(this.collapsed ? "workspaceRibbon.expand" : "workspaceRibbon.collapse"));
    this.scope.bindAttr(this.toggle, "aria-label", () => message(this.collapsed ? "workspaceRibbon.expand" : "workspaceRibbon.collapse"));
    this.toggle.onclick = () => this.setCollapsed(!this.collapsed);
    this.element.append(this.tablist, this.toggle);
    for (const tab of tabs) {
      const button = document.createElement("button"), panel = document.createElement("section");
      button.type = "button";
      button.id = `${prefix}-tab-${tab}`;
      button.className = "workspace-ribbon-tab";
      button.dataset.tab = tab;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-controls", `${prefix}-panel-${tab}`);
      this.scope.bindText(button, message(`workspaceRibbon.${tab}`));
      button.onclick = () => { this.select(tab); this.setCollapsed(false); };
      button.onkeydown = event => this.keydown(event, tab);
      panel.id = `${prefix}-panel-${tab}`;
      panel.className = "workspace-ribbon-panel";
      panel.dataset.tab = tab;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", button.id);
      panel.tabIndex = 0;
      this.entries.set(tab, { button, panel, groups: new Map() });
      this.tablist.append(button);
      this.element.append(panel);
    }
    this.paint();
  }

  get selected(): RibbonTab { return this.current; }

  setGroup(tab: RibbonTab, id: string, label: UiText, nodes: HTMLElement[]): void {
    this.assertMutable(); knownTab(tab);
    if (!id || new Set(nodes).size !== nodes.length) throw new Error("Invalid workspace ribbon group");
    const entry = this.entries.get(tab)!;
    let group = entry.groups.get(id);
    if (!group) {
      const element = document.createElement("section"), content = document.createElement("div"), heading = document.createElement("span");
      const scope = new LocaleScope();
      element.className = "workspace-ribbon-group";
      element.dataset.group = id;
      element.setAttribute("role", "group");
      content.className = "workspace-ribbon-group-content";
      heading.className = "workspace-ribbon-group-label";
      heading.id = `${entry.panel.id}-group-${++this.nextGroupId}`;
      element.setAttribute("aria-labelledby", heading.id);
      element.append(content, heading);
      group = { element, content, label: heading, scope, unbindLabel: () => {} };
      entry.groups.set(id, group);
      entry.panel.append(element);
    }
    group.unbindLabel();
    group.unbindLabel = group.scope.bindText(group.label, label);
    // Do not move an unchanged node: appendChild on an already mounted focused
    // control can blur it. Only explicit additions/removals/reordering touch DOM.
    for (const child of Array.from(group.content.children)) if (!nodes.includes(child as HTMLElement)) child.remove();
    for (const [index, node] of nodes.entries()) {
      if (group.content.children[index] !== node) group.content.insertBefore(node, group.content.children[index] ?? null);
    }
    // A caller may transfer a node from another group; its former group is now
    // empty without losing the node or needing a second ownership registry.
    for (const value of this.entries.values()) for (const current of value.groups.values()) current.element.hidden = current.content.children.length === 0;
  }

  clearGroup(tab: RibbonTab, id: string): void {
    this.assertMutable(); knownTab(tab);
    const groups = this.entries.get(tab)!.groups, group = groups.get(id);
    if (!group) return;
    group.scope.dispose();
    group.element.remove();
    groups.delete(id);
  }

  /** Programmatic selection preserves the collapsed preference and editor
   * focus. Explicit tab clicks expand; keyboard navigation uses roving focus. */
  select(tab: RibbonTab, focus = false): void {
    this.assertMutable(); knownTab(tab);
    this.current = tab;
    this.paint();
    if (focus) this.entries.get(tab)!.button.focus();
  }

  setCollapsed(collapsed: boolean): void {
    this.assertMutable();
    const focusWasInside = collapsed && this.entries.get(this.current)!.panel.contains(document.activeElement);
    this.collapsed = collapsed;
    this.paint();
    if (focusWasInside) this.toggle.focus();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.toggle.onclick = null;
    for (const entry of this.entries.values()) {
      entry.button.onclick = null;
      entry.button.onkeydown = null;
      for (const group of entry.groups.values()) group.scope.dispose();
    }
    this.scope.dispose();
    // Do not dispose caller-owned controls or proposal owners.
    this.element.remove();
  }

  private assertMutable(): void { if (this.disposed) throw new Error("Workspace ribbon is disposed"); }
  private paint(): void {
    this.element.dataset.collapsed = String(this.collapsed);
    for (const [tab, entry] of this.entries) {
      entry.button.setAttribute("aria-selected", String(tab === this.current));
      entry.button.tabIndex = tab === this.current ? 0 : -1;
      entry.panel.hidden = this.collapsed || tab !== this.current;
    }
    this.toggle.setAttribute("aria-expanded", String(!this.collapsed));
    this.toggle.setAttribute("aria-controls", this.entries.get(this.current)!.panel.id);
    this.scope.refresh();
  }
  private keydown(event: KeyboardEvent, tab: RibbonTab): void {
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || event.altKey || event.ctrlKey || event.metaKey) return;
    const index = tabs.indexOf(tab);
    const target = event.key === "ArrowRight" ? tabs[(index + 1) % tabs.length]
      : event.key === "ArrowLeft" ? tabs[(index + tabs.length - 1) % tabs.length]
      : event.key === "Home" ? tabs[0] : event.key === "End" ? tabs[tabs.length - 1] : undefined;
    if (!target) return;
    event.preventDefault();
    this.select(target, true);
  }
}
