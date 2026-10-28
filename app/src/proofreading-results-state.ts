import type { ProofreadFinding, ProofreadResult } from "./proofreading-wire";

export const proofreadingBatchSize = 200;
export type ProofreadingSort = "source" | "rule";

/** Presentation and a bounded user selection only; this never batches author writes. */
export class ProofreadingResultsState {
    private report: ProofreadResult | null = null;
    private loaded = proofreadingBatchSize;
    private selected = new Set<string>();
    private ordered: ProofreadFinding[] | null = null;
    category = "all";
    sort: ProofreadingSort = "source";

    replace(report: ProofreadResult | null): void {
        this.report = report;
        this.loaded = proofreadingBatchSize;
        this.selected.clear();
        this.ordered = null;
        if (report) this.selectVisible();
    }
    setCategory(category: string): void {
        if (this.category === category) return;
        this.category = category;
        this.loaded = proofreadingBatchSize;
        this.ordered = null;
        this.reconcileSelection();
    }
    setSort(sort: ProofreadingSort): void {
        if (this.sort === sort) return;
        this.sort = sort;
        this.ordered = null;
        this.reconcileSelection();
    }
    matching(): readonly ProofreadFinding[] {
        if (!this.ordered) {
            const ordinal = new Map(this.report?.findings.map((finding, index) => [finding, index]));
            const source = (a: ProofreadFinding, b: ProofreadFinding): number => {
                const left = a.sourceRange, right = b.sourceRange;
                if (!left || !right) return left ? -1 : right ? 1 : 0;
                return left.utf16From - right.utf16From || left.utf16To - right.utf16To;
            };
            this.ordered = (this.report?.findings ?? []).filter(finding => this.category === "all" || finding.category === this.category).sort((a, b) =>
                (this.sort === "rule" ? a.ruleId < b.ruleId ? -1 : a.ruleId > b.ruleId ? 1 : 0 : 0)
                || source(a, b) || ordinal.get(a)! - ordinal.get(b)!);
        }
        return this.ordered;
    }
    visible(): readonly ProofreadFinding[] { return this.matching().slice(0, this.loaded); }
    get total(): number { return this.report?.findings.length ?? 0; }
    get remaining(): number { return Math.max(0, this.matching().length - this.loaded); }
    loadMore(): void { this.loaded += Math.min(proofreadingBatchSize, this.remaining); }
    eligible(finding: ProofreadFinding): boolean { return !!finding.sourceRange && !this.report?.existing.includes(finding.id); }
    recorded(finding: ProofreadFinding): boolean { return this.report?.existing.includes(finding.id) ?? false; }
    isSelected(id: string): boolean { return this.selected.has(id); }
    clearSelection(): void { this.selected.clear(); }
    selectVisible(): void {
        this.selected.clear();
        for (const finding of this.visible()) {
            if (this.selected.size === proofreadingBatchSize) break;
            if (this.eligible(finding)) this.selected.add(finding.id);
        }
    }
    select(finding: ProofreadFinding, checked: boolean): boolean {
        this.reconcileSelection();
        if (!this.visible().includes(finding) || !this.eligible(finding)) return false;
        if (!checked) { this.selected.delete(finding.id); return true; }
        if (!this.selected.has(finding.id) && this.selected.size >= proofreadingBatchSize) return false;
        this.selected.add(finding.id);
        return true;
    }
    reconcileSelection(): void {
        const visible = new Set(this.visible().filter(finding => this.eligible(finding)).map(finding => finding.id));
        for (const id of this.selected) if (!visible.has(id)) this.selected.delete(id);
    }
    selectedIds(): string[] {
        this.reconcileSelection();
        return [...new Set(this.visible().filter(finding => this.selected.has(finding.id) && this.eligible(finding)).map(finding => finding.id))];
    }
}
