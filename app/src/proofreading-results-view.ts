import { LocaleScope, displayMessage, message, type UiText } from "./ui-locale";
import { ProofreadingResultsState, proofreadingBatchSize, type ProofreadingSort } from "./proofreading-results-state";
import { proofreadFindingUiText, type ProofreadFinding } from "./proofreading-wire";

interface Host {
    mutable(): boolean;
    selectionLimit(): void;
    annotate(ignore: boolean): void | Promise<void>;
    locate(finding: ProofreadFinding): void;
}
interface Row {
    element: HTMLElement;
    check: HTMLInputElement;
    locate: HTMLButtonElement;
    recorded: HTMLElement;
    scope: LocaleScope;
}
const categoryMessages = { character: message("proofreading.categoryCharacter"), typo: message("proofreading.categoryTypo"), pair: message("proofreading.categoryPair"), sensitive: message("proofreading.categorySensitive") };

/** A report-owned subtree: loading adds rows, while locale updates only text slots. */
export class ProofreadingResultsView {
    readonly element = document.createElement("section");
    private readonly scope = new LocaleScope();
    private readonly count = document.createElement("p");
    private readonly category = document.createElement("select");
    private readonly order = document.createElement("select");
    private readonly list = document.createElement("div");
    private readonly more: HTMLButtonElement;
    private readonly selectVisible: HTMLButtonElement;
    private readonly clear: HTMLButtonElement;
    private readonly annotate: HTMLButtonElement;
    private readonly ignore: HTMLButtonElement;
    private readonly rows = new Map<ProofreadFinding, Row>();
    private busy = false;
    private disposed = false;

    constructor(private readonly state: ProofreadingResultsState, private readonly host: Host) {
        this.element.className = "proofreading-results";
        this.element.setAttribute("role", "region");
        this.scope.bindAttr(this.element, "aria-label", message("proofreading.resultListLabel"));
        this.count.className = "knowledge-help proofreading-result-count";
        this.count.setAttribute("role", "status");
        this.count.setAttribute("aria-live", "polite");
        this.scope.bindText(this.count, () => displayMessage("proofreading.resultCount", { p0: state.visible().length, p1: state.matching().length, p2: state.total, p3: state.selectedIds().length }));
        const controls = document.createElement("div");
        controls.className = "knowledge-actions proofreading-result-controls";
        this.scope.bindAttr(this.category, "aria-label", message("proofreading.allCategories"));
        this.option(this.category, "all", message("proofreading.allCategories"));
        for (const [key, label] of Object.entries(categoryMessages)) this.option(this.category, key, label);
        this.category.onchange = () => { if (!this.canAct()) return; state.setCategory(this.category.value); this.paint(); };
        this.scope.bindAttr(this.order, "aria-label", message("proofreading.sortLabel"));
        this.option(this.order, "source", message("proofreading.sortSource"));
        this.option(this.order, "rule", message("proofreading.sortRule"));
        this.order.onchange = () => { if (!this.canAct()) return; state.setSort(this.order.value as ProofreadingSort); this.paint(); };
        controls.append(this.category, this.order);
        const help = document.createElement("p");
        help.className = "knowledge-help";
        this.scope.bindText(help, message("proofreading.selectionVisibilityHelp"));
        const actions = document.createElement("div");
        actions.className = "knowledge-actions";
        this.selectVisible = this.button(message("proofreading.selectVisible"), () => { state.selectVisible(); this.paint(); });
        this.clear = this.button(message("proofreading.clearSelection"), () => { state.clearSelection(); this.paint(); });
        this.annotate = this.button(message("proofreading.annotateSelected"), () => host.annotate(false));
        this.ignore = this.button(message("proofreading.ignoreSelected"), () => host.annotate(true));
        actions.append(this.selectVisible, this.clear, this.annotate, this.ignore);
        this.list.className = "proofreading-result-list";
        this.list.setAttribute("role", "list");
        this.more = this.button(() => displayMessage("proofreading.showMore", { p0: Math.min(proofreadingBatchSize, state.remaining) }), () => {
            const before = state.visible().length;
            state.loadMore();
            this.paint();
            const firstNew = state.visible()[before];
            if (firstNew) this.rows.get(firstNew)?.element.focus({ preventScroll: true });
        });
        this.element.append(controls, this.count, help, actions, this.list, this.more);
        this.paint();
    }
    private canAct(): boolean {
        return !this.disposed && !this.busy && this.element.isConnected && !this.element.closest("[hidden],[inert]") && this.host.mutable();
    }
    private option(select: HTMLSelectElement, value: string, label: UiText): void {
        const option = document.createElement("option");
        option.value = value;
        this.scope.bindText(option, label);
        select.append(option);
    }
    private button(label: UiText, run: () => void | Promise<void>): HTMLButtonElement {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "knowledge-button";
        this.scope.bindText(button, label);
        button.onclick = () => { if (this.canAct()) void run(); };
        return button;
    }
    private createRow(finding: ProofreadFinding): Row {
        const scope = new LocaleScope(), element = document.createElement("article"), check = document.createElement("input"), label = document.createElement("label"), title = document.createElement("span"), explanation = document.createElement("p"), locate = document.createElement("button"), recorded = document.createElement("p");
        element.className = "knowledge-card proofreading-finding";
        element.dataset.findingId = finding.id;
        element.setAttribute("role", "listitem");
        element.tabIndex = -1;
        check.type = "checkbox";
        check.onchange = () => {
            if (!this.canAct()) { check.checked = this.state.isSelected(finding.id); return; }
            if (!this.state.select(finding, check.checked)) this.host.selectionLimit();
            this.paint();
        };
        label.className = "proofreading-finding-label";
        // Keep exact author text in a separate slot; locale changes do not rebuild a row.
        const category = document.createElement("span"), excerpt = document.createElement("span");
        scope.bindText(category, categoryMessages[finding.category as keyof typeof categoryMessages] ?? finding.category);
        excerpt.textContent = finding.excerpt;
        title.append(category, document.createTextNode(": "), excerpt);
        label.append(check, title);
        scope.bindText(explanation, proofreadFindingUiText(finding));
        element.append(label, explanation);
        if (finding.replacement !== null) {
            const replacement = document.createElement("p");
            scope.bindText(replacement, displayMessage("proofreading.replacementSuggestion", { p0: finding.excerpt, p1: finding.replacement, p2: finding.executable ? "" : message("proofreading.manualFormattingReviewSuffix") }));
            element.append(replacement);
        }
        scope.bindText(recorded, message("proofreading.findingRecorded"));
        element.append(recorded);
        if (!finding.sourceRange) {
            const unavailable = document.createElement("p");
            scope.bindText(unavailable, message("proofreading.findingLocationUnavailable"));
            element.append(unavailable);
        }
        locate.type = "button";
        locate.className = "knowledge-button";
        scope.bindText(locate, message("proofreading.locateFinding"));
        locate.onclick = () => { if (this.canAct()) this.host.locate(finding); };
        locate.hidden = !finding.sourceRange;
        element.append(locate);
        return { element, check, locate, recorded, scope };
    }
    refresh(busy = this.busy): void { this.busy = busy; this.paint(); }
    private paint(): void {
        if (this.disposed) return;
        this.state.reconcileSelection();
        const active = this.element.ownerDocument.activeElement as HTMLElement | null;
        const ownedFocus = active && this.element.contains(active) ? active : null;
        const scrollTop = this.list.scrollTop;
        const visible = this.state.visible(), keep = new Set(visible);
        for (const [finding, row] of this.rows) {
            if (!keep.has(finding)) row.element.remove();
        }
        let cursor = this.list.firstElementChild;
        for (const finding of visible) {
            let row = this.rows.get(finding);
            if (!row) { row = this.createRow(finding); this.rows.set(finding, row); }
            row.check.checked = this.state.isSelected(finding.id);
            row.check.disabled = this.busy || !this.state.eligible(finding);
            row.locate.disabled = this.busy;
            row.recorded.hidden = !this.state.recorded(finding);
            if (row.element === cursor) cursor = cursor.nextElementSibling;
            else this.list.insertBefore(row.element, cursor);
        }
        this.category.value = this.state.category;
        this.order.value = this.state.sort;
        this.category.disabled = this.order.disabled = this.selectVisible.disabled = this.busy;
        const selected = this.state.selectedIds().length;
        this.clear.disabled = this.busy || selected === 0;
        this.annotate.disabled = this.ignore.disabled = this.busy || selected === 0;
        this.more.hidden = this.state.remaining === 0;
        this.more.disabled = this.busy;
        this.scope.refresh();
        this.list.scrollTop = scrollTop;
        if (ownedFocus?.isConnected && this.element.contains(ownedFocus) && this.element.ownerDocument.activeElement !== ownedFocus) ownedFocus.focus({ preventScroll: true });
    }
    focus(): void { this.count.tabIndex = -1; this.count.focus({ preventScroll: true }); }
    dispose(): void {
        if (this.disposed) return;
        this.disposed = true;
        this.scope.dispose();
        for (const row of this.rows.values()) row.scope.dispose();
        this.rows.clear();
        this.element.remove();
    }
}
