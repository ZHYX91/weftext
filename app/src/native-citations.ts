import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { AuthorInputSession } from "./author-input";
import { parseUiMessage, type UiMessage } from "./ui-locale";
import { decodeErrorDescription, descriptionUiText, exactKeys, type ErrorDescription } from "./ui-diagnostics";
import { action, ui } from "./data-panel";
export type CitationForm = "normal" | "prose" | "author" | "year" | "full" | "none";
export interface NativeCitationRange {
    from: number;
    to: number;
    utf16From: number;
    utf16To: number;
    raw: string;
    writable: boolean;
}
export interface NativeCitationSlot {
    key: string;
    bindingProven?: boolean;
    range: NativeCitationRange;
    form?: CitationForm | null;
    formExplicit?: boolean;
    supplement?: string | null;
    supplementExplicit?: boolean;
    style?: string | null;
    styleExplicit?: boolean;
    editable: boolean;
    reason?: ErrorDescription | null;
}
export interface NativeReferenceTarget {
    label: string;
    kind: string;
    range: NativeCitationRange;
    numbered?: boolean;
    editable?: boolean;
    reason?: ErrorDescription | null;
}
export interface NativeCitationSnapshot {
    ownerReference?: string;
    generation: string;
    coverage: "complete" | "partial";
    choices: {
        reference: string;
        title: string|null;
        titlePlaceholder?:UiMessage;
        node?: {
            workspaceId: string;
            nodeId: string;
        };
        citationReady: boolean;
        suggestedKey?: string;
        reason?: ErrorDescription | null;
    }[];
    editor: {
        sourceHash: string;
        bindings: {
            key: string;
            work: string;
        }[];
        citations: NativeCitationSlot[];
        targets: NativeReferenceTarget[];
        crossReferences: {
            label: string;
            kind?: string;
            renderedKind?: string;
            range: NativeCitationRange;
        }[];
        renameAvailable: boolean;
        renameReason?: ErrorDescription | null;
    };
    bindings: {
        key: string;
        work: string;
        state: string;
        authoredSlots: NativeCitationSlot[];
    }[];
    incoming: {
        title: string|null;
        reference: string;
        slots: NativeCitationSlot[];
        visibility: string;
    }[];
    displayStatus: string;
    displayExplanation: UiMessage;
    bibliographyOwnership?: "singleVirtual" | "unavailable";
    links?: {
        range: NativeCitationRange;
        target: string;
        title?: string | null;
        editable: boolean;
        reason?: ErrorDescription|null;
    }[];
    unlabelledTargets?: {
        kind: string;
        range: NativeCitationRange;
        title: string;
    }[];
    displayed?: {
        key: string;
        form?: string | null;
        ordinal: number;
        range?: NativeCitationRange | null;
        visibility?: string;
    }[];
}
export interface NativeCitationInputs {
    sourceHash?: string;
    search: string;
    mode: string;
    work: string;
    key: string;
    form: CitationForm;
    supplement: string;
    text: string;
    target: string;
    newLabel: string;
    referenceForm: string;
    rename: Record<string, string>;
    slots: Record<string, {
        form: CitationForm | "";
        supplement: string;
        supplementChanged: boolean;
        style: string;
        styleChanged?: boolean;
    }>;
}
/** Endpoint-specific protocol decoding; author titles, keys, work identities and ranges stay exact. */
export function decodeNativeCitationSnapshot(value:unknown,scope:"local"|"hosted"="local"):NativeCitationSnapshot {
    const fail=():never=>{throw new UiError(uiMessage("error.invalidResponse"));};
    const object=(value:unknown):Record<string,unknown>|undefined=>value && typeof value==="object" && !Array.isArray(value)?value as Record<string,unknown>:undefined;
    const nonnegative=(value:unknown)=>typeof value==="number" && Number.isSafeInteger(value) && value>=0;
    const range=(value:unknown):NativeCitationRange|undefined=>{const item=object(value);return item && exactKeys(item,["from","to","utf16From","utf16To","raw","writable"]) && [item.from,item.to,item.utf16From,item.utf16To].every(nonnegative) && (item.to as number)>=(item.from as number) && (item.utf16To as number)>=(item.utf16From as number) && typeof item.raw==="string" && typeof item.writable==="boolean" ? {...item} as unknown as NativeCitationRange:undefined;};
    const node=(value:unknown)=>{const item=object(value);return !!item && exactKeys(item,["workspaceId","nodeId"]) && typeof item.workspaceId==="string" && typeof item.nodeId==="string";};
    const reason=(value:unknown):ErrorDescription|null=>value===null?null:decodeErrorDescription(value)??fail();
    const slot=(value:unknown):NativeCitationSlot=>{
        const item=object(value),sourceRange=item && range(item.range);
        if(!item || !sourceRange || !exactKeys(item,["key","bindingProven","range","form","formExplicit","supplement","supplementExplicit","style","styleExplicit","editable","reason"]) || typeof item.key!=="string" || typeof item.bindingProven!=="boolean" || item.form!==null && !["normal","prose","author","year","full","none"].includes(String(item.form)) || [item.formExplicit,item.supplementExplicit,item.styleExplicit,item.editable].some(value=>typeof value!=="boolean") || item.supplement!==null && typeof item.supplement!=="string" || item.style!==null && typeof item.style!=="string")return fail();
        return {...item,range:sourceRange,reason:reason(item.reason)} as unknown as NativeCitationSlot;
    };
    const list=<T>(value:unknown,decode:(value:unknown)=>T):T[]=>{if(!Array.isArray(value))return fail();return value.map(decode);};
    const item=object(value);if(!item || !exactKeys(item,["generation","coverage","choices","editor","bindings","incoming","links","bibliographyOwnership","displayStatus","displayExplanation",...(scope==="local"?["displayed"]:["ownerReference"])],scope==="local"?["ownerReference","unlabelledTargets"]:["displayed","unlabelledTargets"]) || typeof item.generation!=="string" || !["complete","partial"].includes(String(item.coverage)) || !["singleVirtual","unavailable"].includes(String(item.bibliographyOwnership)) || typeof item.displayStatus!=="string" || Object.hasOwn(item,"ownerReference") && typeof item.ownerReference!=="string")return fail();
    const explanation=parseUiMessage(item.displayExplanation);if(!explanation)return fail();
    const choices=list(item.choices,value=>{
        const choice=object(value);if(!choice || !exactKeys(choice,scope==="local"?["reference","title","titlePlaceholder","node","citationReady","suggestedKey","reason"]:["resource","reference","title","citationReady","reason"],scope==="local"?[]:["suggestedKey"]) || typeof choice.reference!=="string" || choice.title!==null && typeof choice.title!=="string" || typeof choice.citationReady!=="boolean" || Object.hasOwn(choice,"suggestedKey") && typeof choice.suggestedKey!=="string" || scope==="local" && !node(choice.node) || scope==="hosted" && (typeof choice.resource!=="string" || typeof choice.title!=="string"))return fail();
        const placeholder=scope==="local"?parseUiMessage(choice.titlePlaceholder):undefined;if(scope==="local" && !placeholder)return fail();
        return {...choice,reason:reason(choice.reason),...(placeholder?{titlePlaceholder:placeholder}:{})} as unknown as NativeCitationSnapshot["choices"][number];
    });
    const editor=object(item.editor);if(!editor || !exactKeys(editor,["sourceHash","bindings","citations","targets","crossReferences","renameAvailable","renameReason"]) || typeof editor.sourceHash!=="string" || typeof editor.renameAvailable!=="boolean")return fail();
    const bindings=list(editor.bindings,value=>{const binding=object(value);if(!binding || !exactKeys(binding,["key","work"]) || typeof binding.key!=="string" || typeof binding.work!=="string")return fail();return {key:binding.key,work:binding.work};});
    const citations=list(editor.citations,slot);
    const targets=list(editor.targets,value=>{const target=object(value),targetRange=target && range(target.range);if(!target || !targetRange || !range(target.labelRange) || !exactKeys(target,["label","kind","range","labelRange","unique","sourceOnly"]) || typeof target.label!=="string" || typeof target.kind!=="string" || typeof target.unique!=="boolean" || typeof target.sourceOnly!=="boolean")return fail();return {...target,range:targetRange} as unknown as NativeReferenceTarget;});
    const crossReferences=list(editor.crossReferences,value=>{const reference=object(value),targetRange=reference && range(reference.range);if(!reference || !targetRange || !exactKeys(reference,["label","kind","range"],["renderedKind"]) || typeof reference.label!=="string" || typeof reference.kind!=="string" || Object.hasOwn(reference,"renderedKind") && typeof reference.renderedKind!=="string")return fail();return {...reference,range:targetRange} as unknown as NativeCitationSnapshot["editor"]["crossReferences"][number];});
    const groups=list(item.bindings,value=>{const group=object(value);if(!group || !exactKeys(group,["key","work","state","authoredSlots"]) || [group.key,group.work,group.state].some(value=>typeof value!=="string"))return fail();return {...group,authoredSlots:list(group.authoredSlots,slot)} as unknown as NativeCitationSnapshot["bindings"][number];});
    const incoming=list(item.incoming,value=>{const entry=object(value);if(!entry || !exactKeys(entry,["title","reference","slots","visibility",...(scope==="local"?["source"]:[])]) || entry.title!==null && typeof entry.title!=="string" || typeof entry.reference!=="string" || typeof entry.visibility!=="string" || scope==="local" && !node(entry.source))return fail();return {...entry,slots:list(entry.slots,slot)} as unknown as NativeCitationSnapshot["incoming"][number];});
    const links=list(item.links,value=>{const link=object(value),sourceRange=link && range(link.range);if(!link || !sourceRange || !exactKeys(link,["range","target","title","editable","reason"]) || typeof link.target!=="string" || link.title!==null && typeof link.title!=="string" || typeof link.editable!=="boolean")return fail();return {...link,range:sourceRange,reason:reason(link.reason)} as unknown as NonNullable<NativeCitationSnapshot["links"]>[number];});
    const displayed=Object.hasOwn(item,"displayed")?list(item.displayed,value=>{const entry=object(value);if(!entry || !exactKeys(entry,["key","form","ordinal","range","originKind","visibility"]) || typeof entry.key!=="string" || typeof entry.form!=="string" || !nonnegative(entry.ordinal) || entry.range!==null && !range(entry.range) || typeof entry.originKind!=="string" || typeof entry.visibility!=="string")return fail();return {...entry} as unknown as NonNullable<NativeCitationSnapshot["displayed"]>[number];}):undefined;
    const unlabelledTargets=Object.hasOwn(item,"unlabelledTargets")?list(item.unlabelledTargets,value=>{const target=object(value),sourceRange=target && range(target.range);if(!target || !sourceRange || !exactKeys(target,["kind","range","title"]) || typeof target.kind!=="string" || typeof target.title!=="string")return fail();return {...target,range:sourceRange} as unknown as NonNullable<NativeCitationSnapshot["unlabelledTargets"]>[number];}):undefined;
    return {...item,choices,editor:{sourceHash:editor.sourceHash,bindings,citations,targets,crossReferences,renameAvailable:editor.renameAvailable,renameReason:reason(editor.renameReason)},bindings:groups,incoming,links,displayExplanation:explanation,...(displayed?{displayed}:{}),...(unlabelledTargets?{unlabelledTargets}:{})} as unknown as NativeCitationSnapshot;
}
export function newNativeCitationInputs(): NativeCitationInputs { return { search: "", mode: "link", work: "", key: "", form: "normal", supplement: "", text: "", target: "", newLabel: "", referenceForm: "normal", rename: {}, slots: {} }; }
export function citationInputScope(context: {
    deployment: string;
    actor: string;
    resource: string;
}, ownerReference?: string): string { return JSON.stringify([context.deployment, context.actor, ownerReference ?? context.resource]); }
export function citationInsertionByte(source: string, utf16: number): number {
    if (!Number.isSafeInteger(utf16) || utf16 < 0 || utf16 > source.length || utf16 > 0 && utf16 < source.length && /[\uD800-\uDBFF]/.test(source[utf16 - 1]) && /[\uDC00-\uDFFF]/.test(source[utf16]))
        throw new UiError(uiMessage("nativeCitation.sourceBoundaryRequired"));
    return new TextEncoder().encode(source.slice(0, utf16)).length;
}
export function citationCandidateKey(data: NativeCitationSnapshot, work: string): {
    key: string;
    reason?: UiText;
} {
    const bindings = data.editor.bindings.filter(binding => binding.work === work);
    if (bindings.length === 1)
        return { key: bindings[0].key };
    if (bindings.length > 1)
        return { key: "", reason: uiMessage("nativeCitation.multipleWorkKeys") };
    const suggested = data.choices.find(choice => choice.reference === work)?.suggestedKey || "work";
    if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(suggested))
        return { key: "", reason: uiMessage("nativeCitation.suggestedKeyInvalid") };
    if (data.editor.bindings.some(binding => binding.key === suggested) || data.editor.targets.some(target => target.label === suggested))
        return { key: "", reason: uiMessage("nativeCitation.suggestedKeyConflict") };
    return { key: suggested };
}
const forms: [
    string,
    UiText
][] = [["normal", uiMessage("nativeCitation.formNormal")], ["prose", uiMessage("nativeCitation.formNarrative")], ["author", uiMessage("nativeCitation.formAuthorOnly")], ["year", uiMessage("nativeCitation.formYearOnly")], ["full", uiMessage("nativeCitation.formFullEntry")], ["none", uiMessage("nativeCitation.formBibliographyOnly")]];
function select(values: [
    string,
    UiText
][], value: string, label: UiText): HTMLSelectElement {
    const control = ui("select", "data-input");
    setUiAttr(control, "aria-label", label);
    for (const [key, text] of values) {
        const option = ui("option", "", text);
        option.value = key;
        control.append(option);
    }
    control.value = value;
    return control;
}
function input(value: string, label: UiText): HTMLInputElement { const control = ui("input", "data-input"); control.value = value; setUiAttr(control, "aria-label", label); setUiAttr(control, "placeholder", label); return control; }
export function renderNativeCitations(container: HTMLElement, data: NativeCitationSnapshot | undefined, state: NativeCitationInputs, host: {
    inputs?: AuthorInputSession;
    writable: boolean;
    at(): number;
    perform(title: UiText, intent: Record<string, unknown>): Promise<unknown>;
    select(range: NativeCitationRange): void;
    navigate(reference: string): void;
    repaint(): void;
}): void {
    if (!data) {
        container.append(ui("p", "hint", uiMessage("nativeCitation.catalogUnavailable")));
        return;
    }
    if (state.sourceHash !== data.editor.sourceHash) {
        state.target = "";
        state.sourceHash = data.editor.sourceHash;
    }
    const run = async (title: UiText, intent: Record<string, unknown>): Promise<void> => { await host.perform(title, { ...intent, generation: data.generation }); };
    container.append(ui("p", "hint", data.coverage === "partial" ? uiMessage("nativeCitation.catalogPartial") : uiMessage("nativeCitation.authorizedScopeRead")));
    const mode = select([["link", uiMessage("nativeCitation.typeNoteLink")], ["citation", uiMessage("nativeCitation.typeFormalCitation")], ["reference", uiMessage("nativeCitation.typeCrossReference")]], state.mode, uiMessage("nativeCitation.insertionType"));
    mode.onchange = () => {
        if (!mode.isConnected)
            return;
        host.inputs?.invalidate("insert");
        state.mode = mode.value;
        host.repaint();
    };
    container.append(mode);
    const form = select(forms, state.form, uiMessage("nativeCitation.citationForm"));
    form.onchange = () => {
        if (form.isConnected === false)
            return;
        state.form = form.value as CitationForm;
    };
    const supplement = input(state.supplement, uiMessage("nativeCitation.supplementPlaceholder"));
    host.inputs?.bind(supplement, "insert", [{ kind: "member", key: "supplement" }]);
    supplement.oninput = () => {
        if (!supplement.isConnected)
            return;
        state.supplement = supplement.value;
    };
    const display = input(state.text, uiMessage("nativeCitation.customDisplayPlaceholder"));
    host.inputs?.bind(display, "insert", [{ kind: "member", key: "display" }]);
    display.oninput = () => {
        if (!display.isConnected)
            return;
        state.text = display.value;
    };
    if (state.mode !== "reference") {
        const search = input(state.search, uiMessage("nativeCitation.searchTargets"));
        search.oninput = () => {
            state.search = search.value;
            const term = search.value.toLocaleLowerCase();
            for (const option of choice.options)
                option.hidden = !!option.value && !option.text.toLocaleLowerCase().includes(term);
        };
        const choiceTitle = (choice:NativeCitationSnapshot["choices"][number]):UiText => choice.title ?? choice.titlePlaceholder ?? uiMessage("nativeCitation.authorizedTargetUnavailable");
        const choices = data.choices.filter(choice => !state.search || uiTextValue(choiceTitle(choice)).toLocaleLowerCase().includes(state.search.toLocaleLowerCase()) || choice.reference === state.work);
        const choice = select([["", uiMessage("nativeCitation.selectTarget")], ...choices.map(choice => [choice.reference, choiceTitle(choice)] as [
                string,
                UiText
            ])], state.work, uiMessage("nativeCitation.targetAria"));
        const reason = ui("p", "hint");
        const key = input(state.key, uiMessage("nativeCitation.nativeKeyPlaceholder"));
        host.inputs?.bind(key, "insert", [{ kind: "member", key: "key" }]);
        key.oninput = () => {
            if (!key.isConnected)
                return;
            state.key = key.value;
        };
        const describe = (): void => { const candidate = data.choices.find(choice => choice.reference === state.work); setUiText(reason, state.mode === "citation" ? !candidate?.citationReady ? candidate?.reason ? descriptionUiText(candidate.reason) : uiMessage("nativeCitation.selectPublishedWork") : citationCandidateKey(data, state.work).reason || uiMessage("nativeCitation.formAndStyleDescription") : uiMessage("nativeCitation.authoredDisplayDescription")); insert.disabled = !host.writable || !candidate || state.mode === "citation" && !candidate.citationReady; };
        choice.onchange = () => {
            if (!choice.isConnected)
                return;
            host.inputs?.invalidate("insert");
            state.work = choice.value;
            const suggestion = citationCandidateKey(data, state.work);
            state.key = suggestion.key;
            host.repaint();
        };
        const insert = action(state.mode === "citation" ? uiMessage("nativeCitation.insertFormalCitation") : uiMessage("nativeCitation.insertNoteLink"), () => run(uiMessage("nativeCitation.insertNativeCitation"), state.mode === "citation" ? { kind: "nativeCitationInsert", at: host.at(), work: state.work, key: state.key, form: state.form, supplement: state.supplement } : { kind: "nativeCitationLink", at: host.at(), target: state.work, text: state.text }));
        container.append(search, choice, reason);
        if (state.mode === "citation")
            container.append(key, form, supplement);
        else
            container.append(display);
        container.append(insert);
        describe();
    }
    else {
        const targets = select([["", uiMessage("nativeCitation.selectLocalTarget")], ...data.editor.targets.map(target => [target.label, `${target.kind} · ${target.label}`] as [
                string,
                UiText
            ]), ...(data.unlabelledTargets ?? []).map(target => [`new:${target.range.from}`, displayMessage("nativeCitation.unlabelledTargetChoice", { p0: uiDisplayParameter(target.kind), p1: uiDisplayParameter(target.title) })] as [
                string,
                UiText
            ])], state.target, uiMessage("nativeCitation.crossReferenceTarget"));
        targets.onchange = () => {
            if (!targets.isConnected)
                return;
            host.inputs?.invalidate("insert");
            state.target = targets.value;
            host.repaint();
        };
        const referenceForm = select([["normal", uiMessage("nativeCitation.crossReferenceNumber")], ["page", uiMessage("nativeCitation.crossReferencePage")], ["text", uiMessage("nativeCitation.crossReferenceCustomText")]], state.referenceForm, uiMessage("nativeCitation.crossReferenceForm"));
        referenceForm.onchange = () => {
            if (referenceForm.isConnected === false)
                return;
            state.referenceForm = referenceForm.value;
        };
        const label = input(state.newLabel, uiMessage("nativeCitation.newLabelPlaceholder"));
        host.inputs?.bind(label, "insert", [{ kind: "member", key: "label" }]);
        label.oninput = () => {
            if (!label.isConnected)
                return;
            state.newLabel = label.value;
        };
        const insert = action(uiMessage("nativeCitation.insertCrossReference"), () => { const target = (data.unlabelledTargets ?? []).find(target => `new:${target.range.from}` === state.target); return run(uiMessage("nativeCitation.insertCrossReference"), target ? { kind: "nativeReferenceInsertWithLabel", at: host.at(), target: target.range, label: state.newLabel, form: state.referenceForm } : { kind: "nativeReferenceInsert", at: host.at(), label: state.target, form: state.referenceForm, text: state.text }); });
        insert.disabled = !host.writable;
        container.append(targets, label, referenceForm, display, ui("p", "hint", uiMessage("nativeCitation.labelAndReferenceDescription")), insert);
        const rename = action(uiMessage("nativeCitation.previewLabelRenameScope"), () => run(uiMessage("nativeCitation.renameAuthorScopeLabel"), { kind: "nativeReferenceRename", old: state.target, new: state.newLabel }));
        rename.disabled = !host.writable || !data.editor.renameAvailable || !state.target || state.target.startsWith("new:");
        container.append(rename, ui("p", "hint", uiMessage("nativeCitation.labelRenameDescription")));
    }
    container.append(ui("h3", "", uiMessage("nativeCitation.bindingsAndSourceCitations")), ui("p", "hint", data.displayExplanation));
    for (const binding of data.bindings) {
        const group = ui("section", "knowledge-section");
        const choice = data.choices.find(choice => choice.reference === binding.work);
        group.append(ui("strong", "", () => `${uiTextValue(binding.key)} · ${uiTextValue(choice?.title || uiMessage("nativeCitation.authorizedTargetUnavailable"))}`), ui("p", "hint", binding.state === "boundOnly" ? uiMessage("nativeCitation.bindingWithoutProvenSource") : displayMessage("nativeCitation.authoredSlotCount", { p0: binding.authoredSlots.length })));
        if (data.displayStatus === "nativeCompileAvailable" && data.bibliographyOwnership === "singleVirtual") {
            const displayed = (data.displayed ?? []).filter(item => item.key === binding.key);
            group.append(ui("p", "hint", displayMessage("nativeCitation.htmlMemberCounts", { p0: displayed.filter(item => item.visibility !== "bibliographyOnly").length, p1: displayed.filter(item => item.visibility === "bibliographyOnly").length })));
            for (const item of displayed) {
                if (item.range)
                    group.append(action(displayMessage("nativeCitation.locateDisplayMainSource", { p0: item.ordinal + 1 }), () => host.select(item.range!)));
            }
        }
        else if (data.bibliographyOwnership === "unavailable")
            group.append(ui("p", "hint", uiMessage("nativeCitation.dynamicOwnershipUnproven")));
        if (choice)
            group.append(action(uiMessage("hostedLinks.openAuthorizedTarget"), () => host.navigate(binding.work)));
        const renameId = `${data.editor.sourceHash}:${binding.key}`;
        const rename = input(state.rename[renameId] ?? "", displayMessage("nativeCitation.renameKeyTo", { p0: uiDisplayParameter(binding.key) }));
        host.inputs?.bind(rename, `rename:${renameId}`);
        rename.oninput = () => {
            if (!rename.isConnected)
                return;
            state.rename[renameId] = rename.value;
        };
        const apply = action(uiMessage("nativeCitation.previewCompleteKeyRename"), () => run(uiMessage("nativeCitation.renameOwnedKey"), { kind: "nativeCitationRename", old: binding.key, new: state.rename[renameId] }));
        apply.disabled = !host.writable || !data.editor.renameAvailable;
        group.append(rename, apply);
        if (data.editor.renameReason)
            group.append(ui("p", "hint", descriptionUiText(data.editor.renameReason)));
        for (const slot of binding.authoredSlots) {
            const id = `${data.editor.sourceHash}:${slot.range.from}:${slot.key}`, values = state.slots[id] ??= { form: "", supplement: slot.supplement ?? "", supplementChanged: false, style: slot.style ?? "" };
            const row = ui("article", "knowledge-card");
            row.append(action(displayMessage("nativeCitation.locateSourceKey", { p0: uiDisplayParameter(slot.key) }), () => host.select(slot.range)), ui("p", "hint", displayMessage("nativeCitation.sourceFormStyle", { p0: uiDisplayParameter(slot.form ?? (slot.formExplicit ? uiMessage("nativeCitation.explicitNativeExpression") : uiMessage("nativeCitation.inheritedSetRule"))), p1: uiDisplayParameter(slot.style ?? (slot.styleExplicit ? uiMessage("nativeCitation.explicitNativeExpression") : uiMessage("nativeCitation.inheritedStyleBibliography"))), p2: uiDisplayParameter(slot.form === "none" ? uiMessage("nativeCitation.bibliographyOnlySuffix") : "") })));
            if (!slot.editable)
                row.append(ui("p", "hint", slot.reason ? descriptionUiText(slot.reason) : uiMessage("nativeCitation.nativeExpressionNoSafeControl")));
            const type = select([["", uiMessage("nativeCitation.keepCurrentOrInherited")], ...forms], values.form, displayMessage("nativeCitation.slotCitationForm", { p0: uiDisplayParameter(slot.key) }));
            const extra = input(values.supplement, displayMessage("nativeCitation.slotSupplement", { p0: uiDisplayParameter(slot.key) }));
            host.inputs?.bind(extra, `citation:${id}:supplement`);
            const update = action(uiMessage("nativeCitation.updateChangedParametersOnly"), () => run(uiMessage("nativeCitation.updateOneCitation"), { kind: "nativeCitationUpdate", range: slot.range, ...(values.form ? { form: values.form } : {}), ...(values.supplementChanged ? { supplement: values.supplement } : {}) }));
            const enabled = (): void => { update.disabled = !host.writable || !slot.editable || !values.form && !values.supplementChanged; };
            type.onchange = () => {
                if (type.isConnected === false)
                    return;
                values.form = type.value as CitationForm | "";
                enabled();
            };
            extra.oninput = () => {
                if (!extra.isConnected)
                    return;
                values.supplement = extra.value;
                values.supplementChanged = true;
                enabled();
            };
            enabled();
            const style = input(values.style, uiMessage("nativeCitation.citationStylePlaceholder"));
            host.inputs?.bind(style, `citation:${id}:style`);
            style.oninput = () => {
                if (!style.isConnected)
                    return;
                values.style = style.value;
                values.styleChanged = true;
            };
            const updateStyle = action(uiMessage("nativeCitation.updateOnlyCitationStyle"), () => run(uiMessage("nativeCitation.updateOneNativeStyle"), { kind: "nativeCitationStyle", range: slot.range, style: values.style }));
            updateStyle.disabled = !host.writable || !slot.editable;
            row.append(type, extra, update, style, updateStyle);
            group.append(row);
        }
        container.append(group);
    }
    container.append(ui("h3", "", uiMessage("nativeCitation.incomingBySourceNote")));
    if (!data.incoming.length)
        container.append(ui("p", "hint", data.coverage === "complete" ? uiMessage("nativeCitation.authoredIncomingUnproven") : uiMessage("nativeCitation.noneFoundScopeIncomplete")));
    for (const source of data.incoming) {
        const details = ui("details");
        details.append(ui("summary", "", displayMessage("nativeCitation.sourceCitationCount", { p0: uiDisplayParameter(source.title), p1: source.slots.length })), action(uiMessage("hostedLinks.openSourceNote"), () => host.navigate(source.reference)));
        for (const slot of source.slots)
            details.append(ui("pre", "knowledge-address", `${slot.key}: ${slot.range.raw}`));
        container.append(details);
    }
    container.append(ui("h3", "", uiMessage("nativeCitation.noteLinkDisplayText")));
    for (const link of data.links ?? []) {
        const row = ui("article", "knowledge-card");
        setUiAttr(row, "title", link.title ? displayMessage("nativeCitation.authorizedCurrentTitle", { p0: uiDisplayParameter(link.title) }) : uiMessage("hostedLinks.targetCurrentlyUnavailable"));
        row.append(action(uiMessage("nativeCitation.locateNativeLink"), () => host.select(link.range)), ui("p", "knowledge-address", link.target), ui("p", "hint", link.title ? displayMessage("nativeCitation.currentTitleOriginalBody", { p0: uiDisplayParameter(link.title) }) : link.reason ? descriptionUiText(link.reason) : uiMessage("nativeCitation.targetUnavailableOldTitleHidden")));
        const update = action(uiMessage("knowledge.updateLinkTextToCurrentTitle"), () => run(uiMessage("nativeCitation.updateSelectedLinkText"), { kind: "nativeCitationLinkText", range: link.range, occurrence: { from: link.range.from, to: link.range.to, target: link.target } }));
        update.disabled = !host.writable || !link.editable;
        row.append(update);
        container.append(row);
    }
    container.append(ui("h3", "", uiMessage("knowledge.inDocumentAndUnclassifiedReferences")));
    for (const reference of data.editor.crossReferences) {
        if (reference.renderedKind === "citation")
            continue;
        container.append(action(displayMessage("graph.locateNode", { p0: uiDisplayParameter(reference.label) }), () => host.select(reference.range)), ui("span", "knowledge-badge", reference.renderedKind === "document" ? uiMessage("knowledge.currentNativeInternalReference") : reference.kind === "link" ? uiMessage("knowledge.internalLinkSourceSlot") : uiMessage("nativeCitation.sourceCandidateTypeUnconfirmed")));
    }
    const retained = Object.entries(state.slots).filter(([key, value]) => !key.startsWith(`${data.editor.sourceHash}:`) && (value.form || value.supplementChanged || value.styleChanged));
    const renamed = Object.entries(state.rename).filter(([key, value]) => !key.startsWith(`${data.editor.sourceHash}:`) && value);
    if (retained.length || renamed.length) {
        const recovery = ui("details");
        recovery.append(ui("summary", "", uiMessage("nativeCitation.unappliedOldSourceInput")), ui("p", "hint", uiMessage("nativeCitation.oldInputRetained")), ui("pre", "knowledge-address", JSON.stringify({ citations: retained.map(([source, value]) => ({ source, form: value.form || undefined, supplement: value.supplementChanged ? value.supplement : undefined, style: value.styleChanged ? value.style : undefined })), renames: renamed }, null, 2)));
        container.append(recovery);
    }
}
