// Actual CaptureFields/Controller, DataPanel and HostedFields lifecycle boundaries.
// Controlled persistence/transport only: no Core save, real ACL or native-host acceptance.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";
const repo = path.resolve(import.meta.dirname, ".."), out = process.argv[2];
const fieldId = "people/life_events";
assert.ok(out && path.isAbsolute(out)); assert.ok(path.relative(repo, out).startsWith(".." + path.sep));
await mkdir(out);
const files = ["scripts/test_birthday_panel_browser.mjs", "app/src/birthday.ts", "app/src/data-view.css", "docs/design/examples/schema.json", "app/src/native-value-form.ts", "app/src/native-control-identity.ts", "app/src/builtin-property-labels.ts", "app/src/ui-locale.ts", "app/src/ui-messages.ts", "assets/ui-messages.json", "app/src/capture-fields.ts", "app/src/capture-controller.ts", "app/src/capture-draft-state.ts", "app/src/data-panel.ts", "app/src/hosted-fields.ts", "app/src/personal-control-input.ts", "app/src/author-input.ts", "app/src/hosted.ts", "app/src/editor.ts"];
const inventory = async () => Promise.all(files.map(async file => ({ path: file, sha256: createHash("sha256").update(await readFile(path.join(repo, file))).digest("hex") })));
const result = { scope: "Actual Capture submission field gate, DataPanel refresh/Apply/Undo, HostedFields source-choice dirtiness/re-read and the unchanged hosted beforeunload handler with real PersonalControlInput; controlled transports, no Core/durable-save/ACL/native acceptance", fieldId, cases: [], observations: {}, pageErrors: [], completed: false, sourceBefore: await inventory() };
const { chromium } = createRequire(import.meta.url)(process.env.WEFTEXT_PLAYWRIGHT_MODULE || "playwright");
const { createServer } = await import(pathToFileURL(path.join(repo, "app/node_modules/vite/dist/node/index.js")).href);
let vite, browser;
async function check(name, run) { const item = { name, passed: false }; result.cases.push(item); try { await run(); item.passed = true; } catch (error) { item.failure = error.stack || String(error); process.exitCode = 1; } }
try {
    vite = await createServer({ root: path.join(repo, "app"), configFile: false, server: { host: "127.0.0.1", port: 0, hmr: false, watch: { ignored: ["**/*"] } } }); await vite.listen();
    browser = await chromium.launch({ executablePath: process.env.WEFTEXT_BROWSER_EXECUTABLE || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true });
    result.browser = browser.version();
    const page = await browser.newPage({ viewport: { width: 1250, height: 1000 } }); page.on("pageerror", error => result.pageErrors.push(error.message));
    await page.route("**/birthday-consumers", route => route.fulfill({ contentType: "text/html", body: '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/src/data-view.css"><style>body{font:14px sans-serif;margin:8px}main>section{margin:12px 0;max-width:100%;box-sizing:border-box}.data-panel{width:min(650px,100%);max-width:100%;max-height:none}#source{height:250px}fieldset{min-width:0}input,textarea,select{max-width:100%;box-sizing:border-box}</style><main><section id="capture"></section><section id="data"></section><section id="source"></section><section id="hosted"></section></main>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/birthday-consumers`);
    const builtin = JSON.parse(await readFile(path.join(repo, "docs/design/examples/schema.json"), "utf8"));
    const hostedSource = await readFile(path.join(repo, "app/src/hosted.ts"), "utf8"), hookStart = hostedSource.indexOf('window.addEventListener("beforeunload", e => {');
    assert.ok(hookStart >= 0);
    const beforeUnloadHook = hostedSource.slice(hookStart, hostedSource.indexOf("\n});", hookStart) + 4);
    await page.evaluate(async ({ builtin, beforeUnloadHook, fieldId }) => {
        const [{ CaptureFields }, { CaptureController }, capture, { PersonalControlInput }, { DataPanel }, { HostedFields }, { SourceEditor }, native, { locale, t, message }] = await Promise.all([
            import("/src/capture-fields.ts"), import("/src/capture-controller.ts"), import("/src/capture-draft-state.ts"), import("/src/personal-control-input.ts"), import("/src/data-panel.ts"), import("/src/hosted-fields.ts"), import("/src/editor.ts"), import("/src/native-value-form.ts"), import("/src/ui-locale.ts"),
        ]);
        locale.set("en-US");
        const text = value => ({ kind: "text", value }), record = values => ({ kind: "record", value: Object.entries(values).map(([key, value]) => ({ key, value })) });
        const field = builtin.fields.find(field => field.id === fieldId);
        const bool = value => ({ kind: "bool", value }), int = value => ({ kind: "int", value: String(value) });
        const calendar = (year, month) => record({ calendar: text("gregorian"), version: text("1"), precision: text(month ? "month" : "year"), year: int(year), ...(month ? {month:int(month)} : {}) });
        const evidence = name => ({kind:"array",value:[record({reference:text("https://example.invalid/"+name),note:text("Source note "+name)})]});
        const initial = {kind:"array",value:[
            record({kind:text("birth"),event_time:calendar(2001),selected:bool(true),note:text("Original birth A 😀"),sources:evidence("a"),unknownEvidence:record({code:text("retain-a")})}),
            record({kind:text("death"),event_time:calendar(2090,9),selected:bool(true),note:text("Selected death is independent"),sources:evidence("death")}),
            record({kind:text("birth"),event_time:record({calendar:text("chinese"),version:text("original"),precision:text("month"),year:int(2004),month:int(7),leap_month:bool(true)}),selected:bool(true),note:text("Original birth B"),sources:evidence("b"),unknownEvidence:record({code:text("retain-b")})}),
        ]};
        const member=(value,key)=>value?.kind==="record"?value.value.find(item=>item.key===key)?.value:undefined;
        const kind = value => value;
        const typeId = "people/person", typeAlias = "person";
        const hash = async source => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)))].map(value => value.toString(16).padStart(2, "0")).join("");
        const fixture = window.fixture = { kind, member, locale, initial, labels: { choose:t(message("birthday.choose")), add:t(message("birthday.add")), edit:t(message("birthday.edit")), conflict:t(message("birthday.conflict")), year:t(message("birthday.year")), month:t(message("birthday.month")), monthDay:t(message("birthday.monthDay")), apply: t(message("queryEdit.applyDraft")), reread: t(message("hostedFields.rereadField")), rebase: t(message("hostedFields.adoptCurrentBasis")), save: t(message("hostedFields.saveField")) } };
        const captureInputs = new PersonalControlInput(document, () => true, () => { throw new Error("Capture must retain through its own sink"); });
        const captureFactory = (owner, current, sink, revision) => captureInputs.session("capture-fixture", 1, owner, current, sink, revision);
        const basis = { schemaRef: "weftext-config://fixture/schema", semanticDigest: "a".repeat(64) }, input = capture.emptyCaptureInput();
        input.typeIds = [typeId]; input.formBasis = basis; input.fields = [{ fieldId: field.id, valueState: { kind: "valid", value: structuredClone(initial) } }];
        let durable = { version: 2, draftId: crypto.randomUUID(), revision: 1, inputRevision: 1, inputCut: "full", currentInput: input, pending: null, lastMutation: { kind: "create", mutationId: crypto.randomUUID(), fingerprint: "b".repeat(64), resultingRevision: 1 } };
        const controller = new CaptureController({ async mutate(route, command) {
            if (route !== "input" || command.expectedRevision !== durable.revision) throw new Error("Controlled Capture input CAS only");
            durable = { ...durable, revision: durable.revision + 1, inputRevision: durable.inputRevision + 1, currentInput: structuredClone(command.input), lastMutation: { kind: "input", mutationId: command.mutationId, fingerprint: "b".repeat(64), resultingRevision: durable.revision + 1 } }; return structuredClone(durable);
        }, async flush() { throw new Error("No Core close proof"); } }, durable);
        fixture.capture = { controller, inputs: captureInputs, fields: new CaptureFields(document.querySelector("#capture"), controller, captureFactory, { schema: builtin, ...basis, allowedFields: [field.id] }), async rebuild() {
            this.fields.collectOwnedRaw(); await controller.drain(); this.fields.dispose(); document.querySelector("#capture").replaceChildren();
            this.fields = new CaptureFields(document.querySelector("#capture"), controller, captureFactory, { schema: builtin, ...basis, allowedFields: [field.id] }); await controller.drain();
        }, gate() { this.fields.collectOwnedRaw(); this.fields.assertSubmittable(); return kind(capture.captureFieldCandidate(controller.state.input.fields[0])); } };
        const encode = value => "// controlled field projection " + JSON.stringify(value) + "\n正文😀\r\n", decode = source => JSON.parse(source.split("\n")[0].slice("// controlled field projection ".length));
        fixture.basis = { inputId: "source-input", sessionId: "session", source: encode(initial), serial: 1, baseId: "base", root: "D:/owned-fixture", path: "D:/owned-fixture/Work.typ", workspaceId: "58c47256-bfce-4d29-a037-b83a3a644fb0", nodeId: "32eb9a12-e752-4bfa-8cdf-65e00f23ac26" };
        fixture.editor = new SourceEditor(fixture.basis.source, (source, inputId) => { fixture.basis = { ...fixture.basis, source, inputId, serial: fixture.basis.serial + 1 }; }, () => {}, undefined, fixture.basis.inputId); fixture.editor.mount(document.querySelector("#source"));
        fixture.localRequests = [];
        fixture.data = new DataPanel({ getDocument: () => fixture.basis, canEdit: () => true, notify() {}, async openDocument() { throw new Error("No navigation"); }, async applyPatch(patch, basis) {
            if (basis.source !== fixture.basis.source) throw new Error("Stale controlled patch"); fixture.editor.apply(patch.from, patch.to, patch.text);
        }, async request(route, body) {
            fixture.localRequests.push({ route, body: structuredClone(body) });
            const schemaSource = JSON.stringify(builtin), read = { schema: builtin, source: schemaSource, revision: "schema", semanticDigest: basis.semanticDigest, builtinDefault: false, canInstall: false };
            if (route === "/api/data/schema") return read;
            if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
            if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
            if (route === "/api/data/project") { if (fixture.projectGate) { fixture.projectWaiting = true; await fixture.projectGate; fixture.projectGate = null; fixture.projectWaiting = false; } return { serial: body.serial, sourceHash: await hash(body.source), schemaHash: await hash(schemaSource), schemaRevision: read.revision, readOnly: false, projection: { qualification: "qualified", id: fixture.basis.nodeId, types: [typeAlias], resolvedTypes: [typeId], unknownFields: [], diagnostics: [], budgetExhausted: false, fields: [{ fieldId: field.id, key: field.key, cell: { state: "value", value: decode(body.source) }, sourceRange: { from: 0, to: body.source.length }, raw: JSON.stringify(decode(body.source)), writable: true, required: false, unbound: [] }] } }; }
            if (route === "/api/data/edit") return { from: 0, to: body.source.length, text: encode(body.value) };
            throw new Error("Unexpected local route " + route);
        } }); document.querySelector("#data").append(fixture.data.element); await fixture.data.refresh();
        const expand = shape => { const value = native.resolveShape(shape, builtin); return value.kind === "record" ? { ...value, members: Object.fromEntries(Object.entries(value.members).map(([key, member]) => [key, { ...member, shape: expand(member.shape) }])) } : value.kind === "list" ? { ...value, items: expand(value.items) } : value.kind === "union" ? { ...value, variants: value.variants.map(expand) } : value; };
        const personalControls = new PersonalControlInput(document, () => true, () => {}), authorInputs = (owner, current, sink, revision) => personalControls.session("hosted-fixture", 1, owner, current, sink, revision);
        let server = structuredClone(initial); fixture.hostedWrites = []; fixture.hostedRegistry = personalControls;
        fixture.hosted = new HostedFields(document.querySelector("#hosted"), async (route, body) => {
            if (route === "fields") return { fields: [{ resource: "work", fieldId: field.id, write: true }] };
            if (route === "fields/read") return { resource: "work", basis: "field-basis", qualification: "staticAuthorizedField", field: { fieldId: field.id, key: field.key, shape: expand(field.shape), nullable: false, required: false, writable: true, cell: { state: "value", value: structuredClone(server) } } };
            if (route === "navigation/registry") throw new Error("No registry rights in this controlled field fixture");
            if (route === "fields/write") { fixture.hostedWrites.push(structuredClone(body)); server = structuredClone(body.value); return { status: "committed", cell: { state: "value", value: server } }; }
            throw new Error("Unexpected hosted route " + route);
        }, undefined, undefined, authorInputs);
        await fixture.hosted.connect("birthday-fixture", "alice", 1); await fixture.hosted.openGranted("work", field.id);
        // Execute the unchanged actual hosted consumer, with clean body and every
        // unrelated operation false. The real field registry is its only dirty input.
        new Function("personalInputs", "personalControls", "structures", "media", "library", "history", "draft", beforeUnloadHook)({ pending: () => false }, personalControls, { hasPending: false }, { hasPending: false }, { hasPending: false }, { hasPending: () => false }, { source: "clean body", baseSource: "clean body", pending: null });
        fixture.beforeUnload = () => { const event = new Event("beforeunload", { cancelable: true }); window.dispatchEvent(event); return event.defaultPrevented; };
        fixture.projectionCurrent = () => !fixture.data.readingProjection && ["inputId","sessionId","source","serial","baseId","path","root","nodeId","workspaceId"].every(key => fixture.data.projectionBasis?.[key] === fixture.basis[key]);
        fixture.readData = () => kind(fixture.data.drafts.get(field.key).value);
        fixture.readHosted = () => kind(fixture.hosted.draft.value);
        fixture.readSource = () => kind(decode(fixture.editor.source));
        fixture.controls = (root, valuePath, role) => [...document.querySelectorAll(root + " [data-control-key]")].filter(control => {
            if (JSON.parse(control.dataset.controlKey)[0] !== field.id) return false;
            const address = native.nativeControlAddress(control);
            return address.identity.fieldId === field.id && JSON.stringify(address.identity.path) === JSON.stringify(valuePath) && (!role || address.role === role);
        }).map(control=>({key:control.dataset.controlKey,role:native.nativeControlAddress(control).role,value:control.value,tag:control.tagName}));
        fixture.readCapture = () => capture.captureFieldCandidate(controller.state.input.fields[0]);
        const customSchema=structuredClone(builtin);customSchema.definitions.LifeEvent.members.kind.shape.values.push("private-kind");
        const customHost=document.createElement("section");customHost.id="custom";document.querySelector("main").append(customHost);
        customHost.append(native.valueEditor(field.shape,structuredClone(initial),customSchema,()=>{},"custom life_events",{fieldId:field.id,path:[]}));
        await controller.drain();
    }, { builtin, beforeUnloadHook, fieldId });
    const labels = await page.evaluate(() => window.fixture.labels);
    const eventPath = (index, member) => [{ kind: "index", index }, { kind: "member", key: "event_time" }, ...(member ? [{ kind: "member", key: member }] : [])];
    const control = async (root, valuePath, role) => {
        const entries = await page.evaluate(({root,valuePath,role})=>window.fixture.controls(root,valuePath,role),{root,valuePath,role});
        assert.equal(entries.length,1,`one exact ${role} input at ${JSON.stringify(valuePath)} in ${root}`);
        return page.locator(root).locator(`[data-control-key=${JSON.stringify(entries[0].key)}]`);
    };
    const choose = (root,index) => page.locator(`${root} .native-birthday [data-birth-index="${index}"]`).getByRole("button",{name:labels.choose,exact:true}).click();
    const read = root => page.evaluate(root=>structuredClone(root==="#capture"?window.fixture.readCapture():root==="#data"?window.fixture.readData():window.fixture.readHosted()),root);
    const initial = await page.evaluate(()=>window.fixture.initial);
    const expected = structuredClone(initial); expected.value[0].value.find(item=>item.key==="selected").value.value=false;
    await check("All actual consumers show one builtin birthday panel, conflict and original year/month precision without inventing date parts",async()=>{
        for(const root of ["#capture","#data","#hosted"]){
            assert.equal(await page.locator(root+" .native-birthday").count(),1);
            const panel=page.locator(root+" .native-birthday");await panel.waitFor({state:"attached"});
            assert.ok((await panel.textContent()).includes(labels.conflict));
            assert.ok((await panel.locator('[data-birth-index="0"]').textContent()).includes(labels.year));
            assert.ok((await panel.locator('[data-birth-index="2"]').textContent()).includes(labels.month));
            assert.equal(await panel.locator('[data-birth-index="1"]').count(),0,"death is not a birthday candidate");
            assert.deepEqual(await read(root),initial);
        }
        result.observations.initial=initial;
    });
    await check("Edit date focuses the original union control; Enter cannot activate the member Remove action in any consumer",async()=>{
        for(const root of ["#capture","#data","#hosted"]){
            const before=await read(root);
            await page.locator(`${root} .native-birthday [data-birth-index="0"]`).getByRole("button",{name:labels.edit,exact:true}).click();
            const variant=await control(root,eventPath(0),"variant");
            assert.equal(await variant.evaluate(node=>document.activeElement===node),true);
            await page.keyboard.press("Enter");await page.keyboard.press("Escape");
            assert.deepEqual(await read(root),before,"keyboard editing must not remove event_time");
        }
    });
    await check("Leap month is explicit in every birthday summary and follows the current UI language",async()=>{
        for(const [language,label] of [["en-US","Leap month"],["zh-CN","闰月"]]){
            await page.evaluate(language=>window.fixture.locale.set(language),language);
            for(const root of ["#capture","#data","#hosted"]){
                assert.equal(await page.locator(`${root} .native-birthday [data-birth-index="2"] .birthday-leap-month`).textContent(),label);
                assert.equal(await page.locator(`${root} .native-birthday [data-birth-index="0"] .birthday-leap-month`).count(),0);
                assert.deepEqual(await read(root),initial);
            }
        }
        await page.evaluate(()=>window.fixture.locale.set("en-US"));
    });
    await check("Hosted clicking only a birthday source marks the real field producer unapplied and blocks the unchanged clean-body beforeunload",async()=>{
        assert.equal(await page.evaluate(()=>window.fixture.beforeUnload()),false);
        await choose("#hosted",2);
        assert.deepEqual(await read("#hosted"),expected);
        result.observations.hostedRegistry=await page.evaluate(()=>({registry:window.fixture.hostedRegistry.inspect(),blocksLeave:window.fixture.beforeUnload()}));
        assert.equal(result.observations.hostedRegistry.blocksLeave,true);
        assert.ok(result.observations.hostedRegistry.registry.producers.some(producer=>producer.owner.kind==="field"&&producer.unapplied&&producer.hasObservedChanges));
        assert.deepEqual(await page.evaluate(()=>window.fixture.hostedWrites),[],"source choice does not send a save");
        await page.locator("#hosted").getByRole("button",{name:labels.reread,exact:true}).click();
        await page.waitForFunction(()=>!window.fixture.hosted.busy);
        assert.deepEqual(await read("#hosted"),expected,"reread retains the exact unsaved author selection");
    });
    await check("Capture explicit selection retains event order, independent selected death, sources, notes and unknown data across real rebuild/submission gate",async()=>{
        await choose("#capture",2);await page.evaluate(()=>window.fixture.capture.rebuild());
        assert.deepEqual(await read("#capture"),expected);
        assert.deepEqual(await page.evaluate(()=>window.fixture.capture.gate()),expected);
        assert.deepEqual((await read("#capture")).value[1],initial.value[1]);
    });
    await check("DataPanel refresh retains selection; explicit Apply alone updates SourceEditor with one real Undo/Redo step",async()=>{
        await choose("#data",2);await page.evaluate(()=>window.fixture.data.refresh());
        assert.deepEqual(await read("#data"),expected);
        assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),initial);
        const property=page.locator('#data .data-property[data-field-id="people/life_events"]');
        await page.evaluate(()=>{window.fixture.projectGate=new Promise(resolve=>window.fixture.releaseProject=resolve);});
        await property.getByRole("button",{name:labels.apply,exact:true}).click();
        await page.waitForFunction(()=>window.fixture.projectWaiting);
        assert.equal(await page.evaluate(()=>window.fixture.projectionCurrent()),false,"delayed projection has not completed Apply");
        await page.evaluate(()=>window.fixture.releaseProject());
        await page.waitForFunction(()=>window.fixture.readSource().value[0].value.find(item=>item.key==="selected").value.value===false);
        await page.waitForFunction(()=>window.fixture.projectionCurrent());
        assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),expected);
        await page.evaluate(()=>window.fixture.editor.undo());assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),initial);
        await page.evaluate(()=>window.fixture.editor.redo());assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),expected);
        await page.evaluate(()=>window.fixture.data.refresh());
        assert.equal(await page.evaluate(()=>window.fixture.projectionCurrent()),true);
    });
    await check("Real event_time branch and month/day inputs edit the selected original record without creating a birthday mirror or year",async()=>{
        await (await control("#data",eventPath(2),"variant")).selectOption("3");
        await (await control("#data",eventPath(2,"month"),"value:int")).fill("2");
        await (await control("#data",eventPath(2,"day"),"value:int")).fill("29");
        const edited=await read("#data"), time=edited.value[2].value.find(item=>item.key==="event_time").value;
        assert.deepEqual(time,{kind:"record",value:[{key:"month",value:{kind:"int",value:"2"}},{key:"day",value:{kind:"int",value:"29"}}]});
        const rest=structuredClone(edited);rest.value[2].value.find(item=>item.key==="event_time").value=initial.value[2].value.find(item=>item.key==="event_time").value;
        assert.deepEqual(rest,expected,"only actual selected event_time changes");
        assert.ok((await page.locator('#data .native-birthday [data-birth-index="2"]').textContent()).includes(labels.monthDay));
        await page.evaluate(()=>window.fixture.data.refresh());assert.deepEqual(await read("#data"),edited);
        assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),expected,"date editing remains unapplied");
        assert.equal(JSON.stringify(edited).includes('"key":"birthday"'),false);
        await page.locator('#data .data-property[data-field-id="people/life_events"]').getByRole("button",{name:labels.apply,exact:true}).click();
        await page.waitForFunction(()=>window.fixture.readSource().value[2].value.find(item=>item.key==="event_time").value.value.some(item=>item.key==="day"));
        await page.waitForFunction(()=>window.fixture.projectionCurrent());
        assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),edited);
        await page.evaluate(()=>window.fixture.editor.undo());assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),expected);
        await page.evaluate(()=>window.fixture.editor.redo());assert.deepEqual(await page.evaluate(()=>window.fixture.readSource()),edited);
        await page.evaluate(()=>window.fixture.data.refresh());
        assert.equal(await page.evaluate(()=>window.fixture.projectionCurrent()),true);
        assert.deepEqual(await read("#data"),edited);
        result.observations.monthDay=edited;
    });
    await check("Invalid raw date input survives blocked source choice/add and a real Capture rebuild, without silent normalization",async()=>{
        const year=await control("#capture",eventPath(0,"year"),"value:int");await year.fill("unknown-year生日😀");
        const before=await read("#capture");await choose("#capture",0);
        await page.locator("#capture .native-birthday").getByRole("button",{name:labels.add,exact:true}).click();
        assert.equal(await year.inputValue(),"unknown-year生日😀");assert.deepEqual(await read("#capture"),before);
        await page.evaluate(()=>window.fixture.capture.rebuild());
        assert.equal(await (await control("#capture",eventPath(0,"year"),"value:int")).inputValue(),"unknown-year生日😀");
        assert.equal(await page.evaluate(()=>{try{window.fixture.capture.gate();return false}catch{return true}}),true);
        await (await control("#capture",eventPath(0,"year"),"value:int")).fill("2001");
    });
    await check("IME composition blocks source choice and adding an event while retaining the original control and typed note",async()=>{
        const notePath=[{kind:"index",index:0},{kind:"member",key:"note"}];
        const note=await control("#capture",notePath,"value:text");
        await note.dispatchEvent("compositionstart",{data:"生日"});await note.fill("Composition生日😀");
        const before=await read("#capture");await choose("#capture",0);
        await page.locator("#capture .native-birthday").getByRole("button",{name:labels.add,exact:true}).click();
        assert.equal(await note.inputValue(),"Composition生日😀");assert.deepEqual(await read("#capture"),before);
        assert.equal(await note.evaluate(node=>node.isConnected),true);
        await note.dispatchEvent("compositionend",{data:"生日"});await page.evaluate(()=>window.fixture.capture.controller.drain());
    });
    await check("Explicit Add creates an unselected birth record with no fabricated event_time and leaves prior records intact",async()=>{
        const before=await read("#capture");await page.locator("#capture .native-birthday").getByRole("button",{name:labels.add,exact:true}).click();
        await page.evaluate(()=>window.fixture.capture.controller.drain());
        const after=await read("#capture");assert.deepEqual(after.value.slice(0,before.value.length),before.value);
        assert.deepEqual(after.value.at(-1),{kind:"record",value:[{key:"kind",value:{kind:"text",value:"birth"}},{key:"selected",value:{kind:"bool",value:false}}]});
        assert.equal(await page.evaluate(()=>document.activeElement?.closest('[data-member-key="event_time"]')?.querySelector('.native-member-heading button')===document.activeElement),true,"absent date focuses its explicit Add control");
        await page.evaluate(()=>window.fixture.capture.rebuild());assert.deepEqual(await read("#capture"),after);
        assert.deepEqual(await page.evaluate(()=>window.fixture.capture.gate()),after);
        result.observations.captureFinal=after;
    });
    await check("Same-name custom life_events schema never acquires builtin birthday behavior",async()=>{
        assert.equal(await page.locator("#custom .native-value-editor").count()>0,true);
        assert.equal(await page.locator("#custom .native-birthday").count(),0);
    });
    await check("Actual birthday panel renders at 390px and 1200px without horizontal overflow",async()=>{
        await page.evaluate(()=>window.fixture.locale.set("zh-CN"));
        for(const width of [390,1200]){
            await page.setViewportSize({width,height:900});
            const panel=page.locator("#capture .native-birthday");await panel.scrollIntoViewIfNeeded();
            const geometry=await panel.evaluate(node=>({width:node.getBoundingClientRect().width,scroll:node.scrollWidth,client:node.clientWidth}));
            assert.ok(geometry.width<=width,JSON.stringify({width,geometry}));assert.ok(geometry.scroll<=geometry.client+1,JSON.stringify({width,geometry}));
            await panel.screenshot({path:path.join(out,`birthday-panel-${width}.png`)});
            await page.screenshot({path:path.join(out,`birthday-consumers-${width}.png`),fullPage:false});
            result.observations[`geometry${width}`]=geometry;
        }
    });
    assert.deepEqual(result.pageErrors,[]);result.sourceAfter=await inventory();assert.deepEqual(result.sourceAfter,result.sourceBefore);result.completed=result.cases.every(item=>item.passed);
} catch(error){result.failure=error.stack||String(error);process.exitCode=1;}
finally{if(browser)await browser.close();if(vite)await vite.close();await writeFile(path.join(out,"result.json"),JSON.stringify(result,null,2)+"\n",{flag:"wx"});}
console.log(JSON.stringify({completed:result.completed,cases:result.cases,observations:result.observations,failure:result.failure,out}));
