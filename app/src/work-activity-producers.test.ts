import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { OutputController } from "./export-state";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { TestDocument, TestElement } from "./dom.test-support";
import { displayMessage, errorUiText, message as uiMessage, setUiText, uiDisplayParameter, UiError, type UiText } from "./ui-locale";
import { workActivity, type WorkActivityEvent } from "./work-activity";

// Execute the production composition methods around the real original-request
// controller, without loading modal CSS/media/bootstrap into the state runner.
function methods(file: string, className: string, names: string[], globals: Record<string, unknown>) {
    const source = ts.createSourceFile(file, readFileSync(new URL(file, import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const owner = source.statements.find(node => ts.isClassDeclaration(node) && node.name?.text === className) as ts.ClassDeclaration;
    assert.ok(owner);
    const selected = owner.members.filter(node => ts.isMethodDeclaration(node) && names.includes(node.name.getText(source)));
    assert.equal(selected.length, names.length);
    const code = ts.transpileModule(`class Composition {${selected.map(node => node.getText(source)).join("\n")}}; globalThis.Composition = Composition;`, {
        compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
    }).outputText;
    const context = vm.createContext({ ...globals, uiMessage, displayMessage, errorUiText, setUiText, uiDisplayParameter, UiError, workActivity });
    vm.runInContext(code, context);
    return context.Composition as new () => Record<string, any>;
}

test("export activity follows the real frozen request through unknown prepare and refuses stale result actions", async t => {
    const document = new TestDocument();
    const node = () => Object.assign(document.createElement("div"), { classList: { remove() {} } });
    const ui = (tag: string, className = "", text?: UiText) => { const value = document.createElement(tag); value.className = className; if (text !== undefined) setUiText(value as unknown as HTMLElement, text); return value; };
    const Composition = methods("./export-panel.ts", "OutputDialog", ["prepare", "outputActivity", "outputResult", "showActivityOriginal"], {
        ui, lossList: () => node(), productPreview: () => node(),
    });
    const requests: unknown[] = [], events: WorkActivityEvent[] = [];
    let fail = true, ids = 0;
    const stop = workActivity.subscribe(event => events.push(event), { replay: false });
    t.after(stop);
    const controller = new OutputController({
        id: () => `activity-output-${++ids}`,
        prepare: async request => {
            requests.push(structuredClone(request));
            if (fail) throw new Error("connection closed before outcome");
            return { id: "activity-file", requestId: request.requestId, status: "prepared", filename: "exact.docx", contentType: "application/octet-stream", length: 0,
                digest: "0".repeat(64), summary: uiMessage("common.unavailable"), losses: [], preview: null, expiresInSeconds: 60 };
        },
        confirm: async () => { throw new Error("No confirmation was requested by the view"); },
        download: async () => { throw new Error("No download was requested by the view"); },
    }, { root: "library", path: "library/note.typ", sessionId: "session", source: "frozen source", baseId: "base", serial: 1, inputId: "input" });
    const reports: unknown[] = [];
    const f = Object.assign(new Composition(), {
        output: controller, closeGuard: new ProposalCloseGuard(), outputStatus: node(), outputPreview: node(),
        acceptOutput: document.createElement("input"), dialog: document.createElement("dialog"),
        legacyAvailable: () => true, drawState() {}, report(error: unknown) { reports.push(error); }, discarded: false,
    });
    await f.prepare();
    assert.equal(controller.uncertain, "prepare");
    const tasks = () => events.filter((event): event is Extract<WorkActivityEvent, { kind: "task" }> => event.kind === "task" && event.task.id === "activity-output-1");
    assert.deepEqual(tasks().map(event => event.task.status), ["running", "waiting"]);
    fail = false;
    await f.prepare();
    assert.equal(ids, 1);
    assert.deepEqual(requests[0], requests[1]);
    assert.equal(reports.length, 1);
    assert.equal(controller.confirmed, false);
    const result = events.find(event => event.kind === "result" && event.result.id === "activity-file");
    assert.ok(result && result.kind === "result");
    assert.equal(result.result.actions?.length, 1, "unconfirmed output exposes details, never download");
    await result.result.actions![0].run();
    assert.equal((f.dialog as TestElement).open, true);
    controller.setDraft({ format: "pdf" });
    assert.throws(() => result.result.actions![0].run(), /原操作|original/i);
    assert.equal(ids, 1, "a result action cannot generate a replacement request");
});

test("recognition cancellation stays terminal and its action cannot follow a different author scope", t => {
    const document = new TestDocument(), events: WorkActivityEvent[] = [];
    const stop = workActivity.subscribe(event => events.push(event), { replay: false });
    t.after(stop);
    const statuses = { cancelled: uiMessage("recognition.statusCancelled") };
    const Composition = methods("./recognition-panel.ts", "RecognitionPanel", ["publishActivity", "showActivityOriginal"], { statuses });
    let identity = "first-library";
    const f = Object.assign(new Composition(), {
        identity, host: { identity: () => identity }, closeGuard: new ProposalCloseGuard(), element: document.createElement("dialog"),
        controller: { busy: false, pendingJobId: "activity-cancelled-job", job: { jobId: "activity-cancelled-job", status: "cancelled" } },
    });
    f.publishActivity();
    const event = events.find(event => event.kind === "task" && event.task.id === "activity-cancelled-job");
    assert.ok(event && event.kind === "task");
    assert.equal(event.task.status, "cancelled");
    assert.equal(events.filter(event => event.kind === "result").length, 0);
    identity = "another-library";
    assert.throws(() => event.task.actions![0].run(), /原操作|original/i);
    assert.equal((f.element as TestElement).open, false);
});
