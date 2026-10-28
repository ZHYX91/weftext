import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import type { OutlineBasis, OutlineSnapshot } from "./document-outline";
import type { HeadingSnapshot, StructureHeading } from "./heading-structure-state";

const basis = (patch: Partial<OutlineBasis> = {}): OutlineBasis => ({ sessionId: "original-session", source: "abcdefghijklmnopqrstuvwxyz", serial: 0, inputId: "input-a", path: "C:/TEST/note.typ", root: "C:/TEST", ...patch });
const snapshot = (): HeadingSnapshot => ({ coverage: "complete", truncated: false, items: [
    { key: "heading", kind: "heading", text: "Native heading", level: 1, generated: false, range: { utf16From: 1, utf16To: 6 } },
    { key: "caption", kind: "figure", nativeKind: "figure", text: "Native caption", level: 0, generated: false, range: { utf16From: 9, utf16To: 14 } },
    { key: "generated", kind: "heading", text: "Generated heading", level: 9, generated: true },
] });

function fixture(initial: OutlineBasis | null = basis()) {
    let current = initial, accepted: OutlineSnapshot | null = initial ? { basis: { ...initial }, snapshot: snapshot() } : null, available = true;
    const records: { view: View; snapshot?: HeadingSnapshot; open: (item: StructureHeading) => void; emptyMessage: string; documentTitle?: string }[] = [];
    const views: View[] = [], selections: number[][] = [], revealed: StructureHeading[] = [], notices: string[] = [];
    let requests = 0, clears = 0;
    class View {
        state = { search: "", collapsed: new Set<string>() };
        detached = 0; cleared = 0;
        constructor() { views.push(this); }
        render(_container: HTMLElement, product: HeadingSnapshot | undefined, open: (item: StructureHeading) => void, emptyMessage: string, documentTitle?: string) { records.push({ view: this, snapshot: product, open, emptyMessage, documentTitle }); }
        detach() { this.detached++; }
        clear() { this.cleared++; }
    }
    const source = readFileSync(new URL("./document-outline.ts", import.meta.url), "utf8"), exports: Record<string, any> = {};
    runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText, {
        exports,
        require: (id: string) => id === "./heading-structure-view" ? { HeadingStructureView: View }
            : id === "./ui-locale" ? { message: (key: string) => key, errorUiText: (error: unknown) => String(error) } : {},
    });
    const host = {
        getDocument: () => current, getOutlineSnapshot: () => accepted, canNavigate: () => available,
        selectSource: (from: number, to: number) => selections.push([from, to]),
        revealHeading: (item: StructureHeading) => { revealed.push(item); return true; },
        notify: (message: string) => notices.push(message),
        request: () => { requests++; throw new Error("Outline cannot read or compile"); },
        beforeAction: () => { requests++; throw new Error("Outline cannot settle author input"); },
    };
    const controller = new exports.DocumentOutlineView({ replaceChildren() { clears++; } } as unknown as HTMLElement, host);
    return { controller, records, views, selections, revealed, notices, sameBasis: exports.sameOutlineBasis,
        current: () => current, accepted: () => accepted, requests: () => requests, clears: () => clears,
        setCurrent(next: OutlineBasis | null) { current = next; },
        accept(product: HeadingSnapshot, captured: OutlineBasis = current!) { accepted = { basis: { ...captured }, snapshot: product }; },
        dropProjection() { accepted = null; }, freeze() { available = false; }, thaw() { available = true; },
    };
}

test("local outline consumes the accepted native product without reading, compiling or settling author input", () => {
    const f = fixture(), installed = f.records.at(-1)!;
    assert.equal(installed.snapshot, f.accepted()!.snapshot);
    // The view supplies a copied tree entry. Only the original native item
    // determines the address, even if a UI payload supplies a different range.
    installed.open({ ...installed.snapshot!.items[1], range: { utf16From: 0, utf16To: 26 } });
    assert.deepEqual(f.selections, [[9, 14]]);
    assert.equal(f.requests(), 0);
    assert.equal(f.controller.assertCloseReady, undefined);
});

test("document root gets the captured filename as display text without changing the native heading identity", () => {
    const f = fixture(basis({ path: "C:\\TEST\\Native title.typ" })), record = f.records.at(-1)!;
    assert.equal(record.documentTitle, "Native title.typ");
    assert.equal(record.snapshot, f.accepted()!.snapshot);
    assert.equal(record.snapshot!.items[0].key, "heading");
    assert.equal(f.requests(), 0);
});

test("source edits invalidate immediately and the next accepted projection renews automatically", () => {
    const f = fixture(), old = f.records.at(-1)!;
    f.setCurrent(basis({ source: "new source with enough characters", serial: 1, inputId: "input-b" }));
    f.controller.refresh(); assert.equal(f.records.at(-1)!.snapshot, undefined);
    old.open(old.snapshot!.items[0]); assert.deepEqual(f.selections, []);
    assert.equal(f.notices.at(-1), "heading.outlineBasisChanged");
    const next = snapshot(); f.accept(next); f.controller.refresh();
    const renewed = f.records.at(-1)!; assert.equal(renewed.snapshot, next);
    renewed.open(next.items[0]); assert.deepEqual(f.selections, [[1, 6]]);
    assert.equal(f.requests(), 0);
});

test("ABA source text and reused render keys never revive a stale source address", () => {
    const f = fixture(), old = f.records.at(-1)!, original = f.current()!;
    f.setCurrent({ ...original, source: "intermediate", serial: 1, inputId: "input-b" }); f.controller.invalidate();
    f.setCurrent({ ...original, serial: 2, inputId: "input-c" }); f.controller.refresh();
    assert.equal(f.records.at(-1)!.snapshot, undefined);
    old.open(old.snapshot!.items[0]); assert.deepEqual(f.selections, []);
    const next = snapshot(); next.items[0].range = { utf16From: 4, utf16To: 8 };
    f.accept(next); f.controller.refresh();
    old.open(old.snapshot!.items[0]); assert.deepEqual(f.selections, []);
    f.records.at(-1)!.open(next.items[0]); assert.deepEqual(f.selections, [[4, 8]]);
});

test("an input identity change invalidates a byte-identical projection even without a serial change", () => {
    const f = fixture(), old = f.records.at(-1)!;
    f.setCurrent({ ...f.current()!, inputId: "new-input-with-same-bytes" }); f.controller.refresh();
    assert.equal(f.records.at(-1)!.snapshot, undefined);
    old.open(old.snapshot!.items[0]); assert.deepEqual(f.selections, []);
});

test("a replacement accepted product retires prior callbacks even with an identical basis", () => {
    const f = fixture(), old = f.records.at(-1)!, next = snapshot();
    next.items[1].range = { utf16From: 15, utf16To: 19 };
    f.accept(next); old.open(old.snapshot!.items[1]);
    assert.deepEqual(f.selections, []);
    assert.equal(f.records.at(-1)!.snapshot, next, "the rejected callback also refreshes the accepted presentation");
    f.records.at(-1)!.open(next.items[1]); assert.deepEqual(f.selections, [[15, 19]]);
});

test("every document basis coordinate gates native navigation and capture is immutable", () => {
    const f = fixture(), original = f.current()!;
    assert.equal(f.sameBasis(original, { ...original }), true);
    for (const patch of [{ sessionId: "other" }, { source: "other" }, { serial: 5 }, { inputId: "other" }, { path: "C:/TEST/other.typ" }, { root: "C:/other" }]) assert.equal(f.sameBasis(original, { ...original, ...patch }), false);
    assert.equal(f.sameBasis(original, null), false);
    const old = f.records.at(-1)!;
    // A bad host cannot mutate a captured stamp to relabel an old product.
    f.accepted()!.basis.inputId = "mutated"; f.setCurrent({ ...original, inputId: "mutated" });
    old.open(old.snapshot!.items[0]); assert.deepEqual(f.selections, []);
});

test("switching documents retains per-session navigation state without retaining old presentation DOM", () => {
    const f = fixture(), first = f.records.at(-1)!, original = f.current()!, originalProduct = first.snapshot!;
    first.view.state.search = "kept filter"; first.view.state.collapsed.add("heading");
    f.setCurrent(basis({ sessionId: "second-session", inputId: "second-input", path: "C:/TEST/second.typ" })); f.accept(snapshot()); f.controller.refresh();
    assert.notEqual(f.records.at(-1)!.view, first.view); assert.equal(first.view.detached, 1);
    first.open(originalProduct.items[0]); assert.deepEqual(f.selections, []);
    f.setCurrent(original); f.accept(originalProduct); f.controller.refresh();
    const returned = f.records.at(-1)!;
    assert.equal(returned.view, first.view); assert.equal(returned.view.state.search, "kept filter"); assert.equal(returned.view.state.collapsed.has("heading"), true);
});

test("invalid or ambiguous native ranges cannot fall through to another item", () => {
    for (const range of [{ utf16From: -1, utf16To: 3 }, { utf16From: 3, utf16To: 100 }, { utf16From: 6, utf16To: 1 }, { utf16From: 1.5, utf16To: 3 }]) {
        const f = fixture(), product = snapshot(); product.items[0].range = range; f.accept(product); f.controller.refresh();
        f.records.at(-1)!.open(product.items[0]); assert.deepEqual(f.selections, []); assert.equal(f.notices.at(-1), "error.invalidResponse");
    }
    const f = fixture(), product = snapshot(); product.items.push({ ...product.items[0] }); f.accept(product); f.controller.refresh();
    f.records.at(-1)!.open(product.items[0]); assert.deepEqual(f.selections, []); assert.equal(f.notices.at(-1), "error.invalidResponse");
});

test("generated items use their native display check and frozen or disposed views cannot navigate", () => {
    const f = fixture(), rendered = f.records.at(-1)!, generated = rendered.snapshot!.items[2];
    rendered.open({ ...generated, range: { utf16From: 0, utf16To: 20 } });
    assert.equal(f.revealed[0], generated); assert.deepEqual(f.selections, []);
    f.freeze(); rendered.open(rendered.snapshot!.items[0]); assert.deepEqual(f.selections, []);
    f.thaw(); f.controller.dispose(); rendered.open(rendered.snapshot!.items[0]); assert.deepEqual(f.selections, []);
    assert.equal(f.views.every(view => view.cleared === 1), true); assert.equal(f.clears(), 1);
});

test("closed documents and missing accepted projections have truthful automatic-update states", () => {
    const f = fixture(null); assert.equal(f.records.at(-1)!.emptyMessage, "heading.openDocumentForOutline");
    f.setCurrent(basis()); f.controller.refresh(); assert.equal(f.records.at(-1)!.emptyMessage, "heading.waitForCurrentProjection");
    const product = snapshot(); f.accept(product); f.controller.refresh(); assert.equal(f.records.at(-1)!.snapshot, product);
    const count = f.records.length; f.controller.refresh(); assert.equal(f.records.length, count, "same native product does not rebuild navigation inputs");
    f.dropProjection(); f.controller.refresh(); assert.equal(f.records.at(-1)!.snapshot, undefined);
    assert.equal(f.requests(), 0);
});

test("repeated pending refreshes preserve presentation until source identity or native product changes", () => {
    const f = fixture(); f.dropProjection(); f.controller.refresh();
    const pending = f.records.at(-1)!, pendingCount = f.records.length;
    for (let index = 0; index < 5; index++) { f.controller.refresh(); f.controller.invalidate(); }
    assert.equal(f.records.length, pendingCount, "chrome refreshes do not recreate pending toolbar/filter DOM");
    assert.equal(f.records.at(-1), pending);
    const original = f.current()!;
    f.setCurrent({ ...original, inputId: "new-input" }); f.controller.refresh();
    assert.equal(f.records.length, pendingCount + 1, "a byte-identical new author input retires the old presentation");
    f.controller.refresh(); assert.equal(f.records.length, pendingCount + 1);
    f.setCurrent({ ...f.current()!, source: "new current source", serial: 1 }); f.controller.refresh();
    assert.equal(f.records.length, pendingCount + 2);
    const next = snapshot(); f.accept(next); f.controller.refresh();
    assert.equal(f.records.length, pendingCount + 3); assert.equal(f.records.at(-1)!.snapshot, next);
    f.controller.refresh(); assert.equal(f.records.length, pendingCount + 3);
    f.accept(snapshot()); f.controller.refresh();
    assert.equal(f.records.length, pendingCount + 4, "a new accepted product object refreshes even with an identical basis");
});

test("pending idempotence does not reuse detached, cleared or disposed presentation", () => {
    const f = fixture(null), emptyCount = f.records.length;
    f.controller.refresh(); f.controller.invalidate(); assert.equal(f.records.length, emptyCount);
    const firstBasis = basis(); f.setCurrent(firstBasis); f.controller.refresh(); const first = f.records.at(-1)!, firstCount = f.records.length;
    f.setCurrent(basis({ sessionId: "second", inputId: "second-input", path: "C:/TEST/second.typ" })); f.controller.refresh();
    assert.equal(first.view.detached, 1); assert.equal(f.records.length, firstCount + 1);
    f.setCurrent(firstBasis); f.controller.refresh();
    assert.equal(f.records.at(-1)!.view, first.view); assert.equal(f.records.length, firstCount + 2, "returning to a pending document remounts its detached view");
    f.setCurrent(null); f.controller.refresh(); assert.equal(f.records.at(-1)!.emptyMessage, "heading.openDocumentForOutline");
    f.setCurrent(firstBasis); f.controller.refresh(); assert.equal(f.records.at(-1)!.view, first.view);
    const count = f.records.length; f.controller.dispose(); f.controller.refresh(); f.controller.invalidate(); assert.equal(f.records.length, count);
    assert.equal(f.views.every(view => view.cleared === 1), true);
});
