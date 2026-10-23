import { disposeUiTree, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, ui } from "./data-panel";
import { nativeStyleControlInput, nativeStyleControlReason, type NativeStyleControl, type NativeStyleValue } from "./style-panel";
import { decodeProcessingDocument, defaultProcessingDocument } from "./processing-settings-panel";
import { parseProcessingLanguages, processingLanguageTag } from "./processing-settings";
import { hostedCachePrefix } from "./hosted-state";
import { nodeSourceHash } from "./hosted-navigation-state";
import { movedSibling } from "./node-contents-state";
import { decodeLibraryDraft, freezeLibraryPrepare, libraryCanCommit, libraryInput, libraryReceipt, libraryRoles, libraryTerminal, type LibraryDraft, type LibraryProfile, type LibraryReceipt, type LibraryRole } from "./hosted-library-state";
import { descriptionUiText } from "./ui-diagnostics";
import { decodeLibraryCatalog, decodeLibraryProfile, invalidStyleWire, type LibraryCatalog } from "./style-wire";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
interface HostedOrder {
    parentRef: string;
    settingsSourceHash: string;
    orderedRefs: string[];
    members: {
        targetRef: string;
        name: string;
    }[];
    complete: boolean;
}
const labelSource = uiMessage("hostedLibrary.fullConfigurationSource");
/** Role drafts are proposals; only the original Core operation installs bytes. */
export class HostedLibrary {
    private epoch = 0;
    private inputScopes: AuthorInputScopes;
    private authorSession(draft: LibraryDraft) {
        const role = this.role, epoch = this.epoch;
        if (!role)
            return;
        return this.inputScopes.get(draft, { kind: "library", role }, () => this.epoch === epoch && this.draft === draft && this.role === role, () => draft.revision);
    }
    private prefix = "";
    private catalog: LibraryCatalog = { roles: [], styleUse: false, reorderNodes: false, orderParents: [] };
    private role?: LibraryRole;
    private profile?: LibraryProfile;
    private verifiedOperation?: string;
    private draft?: LibraryDraft;
    private unreadable?: string;
    private busy = false;
    private pendingRoles = new Set<LibraryRole>();
    private picker = ui("select");
    private editor = ui("section");
    private actions = ui("section");
    private message = ui("p", "hint");
    private source = ui("textarea");
    private enabled = ui("input");
    private comparison?: HTMLButtonElement;
    private inputs: {
        control: NativeStyleControl;
        input: HTMLInputElement | HTMLSelectElement;
    }[] = [];
    private orderParent = "";
    private order?: HostedOrder;
    private orderHost = ui("section");
    constructor(private host: HTMLElement, private request: Request, private changed: (role: LibraryRole) => void | Promise<void>, authorInputs?: AuthorInputFactory) { this.inputScopes = new AuthorInputScopes(authorInputs); this.clear(false); }
    get hasPending() { return !!this.draft?.pending || this.pendingRoles.size > 0; }
    clear(remove = true) {
        this.inputScopes.clear();
        this.epoch++;
        this.catalog = { roles: [], styleUse: false, reorderNodes: false, orderParents: [] };
        this.role = undefined;
        this.profile = undefined;
        this.verifiedOperation = undefined;
        this.draft = undefined;
        this.unreadable = undefined;
        this.busy = false;
        this.pendingRoles.clear();
        this.prefix = "";
        this.order = undefined;
        this.orderParent = "";
        for (const node of [this.host, this.editor, this.actions, this.picker, this.message, this.source, this.enabled, this.orderHost]) disposeUiTree(node);
        this.orderHost.replaceChildren();
        this.host.replaceChildren();
        this.host.hidden = true;
        this.editor.replaceChildren();
        this.actions.replaceChildren();
        this.picker.replaceChildren();
        setUiText(this.message, "");
        this.source.value = "";
        this.enabled.checked = false;
        this.inputs = [];
        this.comparison = undefined;
        const failures: unknown[] = [];
        if (remove) {
            try {
                for (const key of Object.keys(sessionStorage))
                    if (key.startsWith("weftext-hosted-library:")) {
                        try {
                            sessionStorage.removeItem(key);
                        }
                        catch (error) {
                            failures.push(error);
                        }
                    }
            }
            catch (error) {
                failures.push(error);
            }
        }
        if (failures.length)
            throw new UiError(uiMessage("hostedLibrary.recoveryCacheRemovalFailed"));
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        const epoch = this.epoch, incoming = await this.request<unknown>("library");
        if (epoch !== this.epoch)
            return;
        const catalog = decodeLibraryCatalog(incoming);
        this.catalog = catalog;
        this.prefix = hostedCachePrefix("library", deployment, actor, generation);
        this.host.hidden = !catalog.roles.length && !catalog.styleUse;
        for (const item of catalog.roles) {
            const raw = sessionStorage.getItem(this.prefix + item.role);
            if (raw) {
                try {
                    if (decodeLibraryDraft(raw, item.role).pending)
                        this.pendingRoles.add(item.role);
                }
                catch { /* The selected role shows the original unreadable record. */ }
            }
        }
        setUiAttr(this.picker, "aria-label", uiMessage("hostedLibrary.configurationRole"));
        this.picker.replaceChildren(...catalog.roles.filter(item => item.read).map(item => { const option = ui("option", "", item.title); option.value = item.role; return option; }));
        this.picker.onchange = () => { void this.run(async () => this.select(this.picker.value as LibraryRole)); };
        this.host.replaceChildren(ui("h2", "", uiMessage("hostedLibrary.configurationAndDefaultStyle")), ui("p", "hint", uiMessage("hostedLibrary.configurationSeparateFromBody")), ui("p", "hint", catalog.styleUse ? uiMessage("hostedLibrary.styleExecutionGranted") : uiMessage("hostedLibrary.styleExecutionNotGranted")), this.picker, this.message, this.editor, this.actions);
        const savedRole = sessionStorage.getItem(this.prefix + "selection"), role = catalog.roles.find(item => item.read && item.role === savedRole)?.role ?? catalog.roles.find(item => item.read)?.role;
        if (role)
            this.select(role);
        else {
            this.picker.hidden = true;
            this.editor.replaceChildren();
            this.actions.replaceChildren();
        }
    }
    private key() { return this.prefix + this.role; }
    private retain() {
        if (this.role && this.draft) {
            sessionStorage.setItem(this.key(), JSON.stringify(this.draft));
            if (this.draft.pending)
                this.pendingRoles.add(this.role);
            else
                this.pendingRoles.delete(this.role);
        }
    }
    private select(role: LibraryRole) {
        if (!this.catalog.roles.some(item => item.role === role && item.read))
            throw new UiError(uiMessage("hostedLibrary.roleUnreadable"));
        try {
            this.retain();
        }
        catch (error) {
            if (this.role)
                this.picker.value = this.role;
            throw error;
        }
        this.inputScopes.clear();
        this.role = role;
        this.picker.hidden = false;
        this.picker.value = role;
        this.profile = undefined;
        this.verifiedOperation = undefined;
        this.draft = undefined;
        this.unreadable = undefined;
        disposeUiTree(this.editor);
        disposeUiTree(this.actions);
        disposeUiTree(this.orderHost);
        this.editor.replaceChildren();
        this.actions.replaceChildren();
        this.source.value = "";
        this.inputs = [];
        this.comparison = undefined;
        this.order = undefined;
        sessionStorage.setItem(this.prefix + "selection", role);
        const raw = sessionStorage.getItem(this.key());
        if (raw !== null) {
            try {
                this.draft = decodeLibraryDraft(raw, role);
            }
            catch {
                this.unreadable = raw;
            }
        }
        setUiText(this.message, this.draft?.pending ? uiMessage("hostedLibrary.originalOperationNeedsCheck") : uiMessage("hostedLibrary.selectRoleReadExactConfiguration"));
        this.render();
    }
    private async run(work: () => void | Promise<void>) {
        if (this.busy)
            return;
        const epoch = this.epoch;
        this.busy = true;
        this.renderActions();
        this.picker.disabled = true;
        try {
            await work();
        }
        catch (error) {
            if (epoch === this.epoch)
                setUiText(this.message, error instanceof Error ? errorUiText(error) : uiMessage("hostedLibrary.operationIncompleteInputRetained"));
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.picker.disabled = false;
                this.renderActions();
                if (this.role === "settings")
                    this.renderOrder();
            }
        }
    }
    private async read() {
        const role = this.role, epoch = this.epoch, revision = this.draft?.revision;
        if (!role || this.unreadable !== undefined)
            return;
        const incoming = await this.request<unknown>("library/read", { role });
        if (epoch !== this.epoch || role !== this.role)
            return;
        const profile = decodeLibraryProfile(incoming);
        if (profile.role !== role)
            throw new UiError(uiMessage("hostedLibrary.readRoleOrSourceMismatch"));
        const draft = this.draft;
        this.profile = profile;
        this.order = undefined;
        if (!draft || draft.revision === revision && !draft.pending && draft.source === draft.baseSource && !Object.keys(draft.changes).length && draft.enabled === draft.baseEnabled)
            this.draft = { source: profile.source, baseSource: profile.source, changes: {}, enabled: profile.enabled, baseEnabled: profile.enabled, revision: (draft?.revision ?? 0) + 1, lastReceipt: draft?.lastReceipt };
        else if (draft.source === profile.source && !Object.keys(draft.changes).length && draft.enabled === profile.enabled && !draft.pending) {
            draft.baseSource = profile.source;
            draft.baseEnabled = profile.enabled;
        }
        setUiText(this.message, this.needsComparison() ? uiMessage("hostedLibrary.serverBasisChangedCompare") : uiMessage("hostedLibrary.exactConfigurationRead"));
        this.render();
        try {
            this.retain();
        }
        catch {
            throw new UiError(uiMessage("hostedLibrary.readButRecoveryUnavailable"));
        }
    }
    private needsComparison() { return !!this.draft && !!this.profile && (this.draft.baseSource !== this.profile.source || this.draft.baseEnabled !== this.profile.enabled); }
    private edited(keepOrder = false) {
        if (!this.draft)
            return;
        this.draft.revision++;
        if (this.role === "settings" && !keepOrder) {
            this.order = undefined;
            this.renderOrder();
        }
        try {
            this.retain();
            setUiText(this.message, this.draft.pending ? uiMessage("hostedLibrary.newInputOriginalRequestFrozen") : uiMessage("hostedLibrary.inputRetainedNotPrepared"));
        }
        catch {
            setUiText(this.message, uiMessage("hostedLibrary.inputRecoveryRequiredBeforeSend"));
        }
        const advanced = this.draft.source !== this.draft.baseSource;
        for (const { control, input } of this.inputs)
            input.disabled = !this.writable() || !control.editable || advanced;
        this.renderActions();
    }
    private writable() { return this.catalog.roles.some(item => item.role === this.role && item.write); }
    private async prepare() {
        const draft = this.draft, profile = this.profile, role = this.role;
        if (!draft || !profile || !role || !this.writable())
            throw new UiError(uiMessage("hostedLibrary.readWritableConfigurationFirst"));
        const request = freezeLibraryPrepare(role, profile, draft, crypto.randomUUID());
        draft.pending = { request, input: libraryInput(draft), revision: draft.revision };
        try {
            this.retain();
        }
        catch (error) {
            delete draft.pending;
            throw new UiError(displayMessage("hostedLibrary.originalOperationPersistenceFailed", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) }));
        }
        this.renderActions();
        await this.sendPrepare();
    }
    private async sendPrepare() {
        const draft = this.draft, pending = draft?.pending, epoch = this.epoch;
        if (!pending)
            return;
        this.retain();
        const result = await this.request<LibraryReceipt>("library/prepare", structuredClone(pending.request));
        if (epoch === this.epoch && this.draft === draft)
            await this.install(result);
    }
    private async operation(settle = false) {
        const draft = this.draft, pending = draft?.pending, epoch = this.epoch;
        if (!pending)
            return;
        this.retain();
        const result = await this.request<LibraryReceipt>(settle ? "library/settle" : "library/operation", { role: pending.request.role, operationId: pending.request.operationId });
        if (epoch === this.epoch && this.draft === draft)
            await this.install(result);
    }
    private async commit() {
        const draft = this.draft, pending = draft?.pending, epoch = this.epoch;
        if (!draft || !pending || this.verifiedOperation !== pending.request.operationId || !libraryCanCommit(draft) || !this.writable() || this.needsComparison())
            throw new UiError(uiMessage("hostedLibrary.checkPreparedPreviewContextChanged"));
        if (pending.commit && (pending.commit.role !== pending.request.role || pending.commit.operationId !== pending.request.operationId || pending.commit.planId !== pending.receipt!.planId)) invalidStyleWire();
        pending.commit ??= { role: pending.request.role, operationId: pending.request.operationId, planId: pending.receipt!.planId! };
        this.retain();
        const result = await this.request<LibraryReceipt>("library/commit", structuredClone(pending.commit));
        if (epoch === this.epoch && this.draft === draft)
            await this.install(result);
    }
    private async install(incoming: LibraryReceipt) {
        const draft = this.draft, pending = draft?.pending;
        if (!draft || !pending)
            return;
        const receipt = libraryReceipt(pending.request, incoming, pending.receipt, draft.baseSource, pending.planId);
        pending.receipt = receipt;
        if (receipt.planId) pending.planId = receipt.planId;
        this.verifiedOperation = pending.request.operationId;
        if (!libraryTerminal(receipt.status)) {
            this.retain();
            setUiText(this.message, receipt.status === "prepared" ? uiMessage("hostedLibrary.planPreparedCheckExactSource") : displayMessage("hostedLibrary.originalOperationStatus", { p0: uiDisplayParameter(receipt.status) }));
            this.renderActions();
            return;
        }
        const current = pending.revision === draft.revision && JSON.stringify(pending.input) === JSON.stringify(libraryInput(draft));
        if (receipt.status === "committed" && current && typeof receipt.preview?.afterSource === "string") {
            draft.source = receipt.preview.afterSource;
            draft.changes = {};
            if (receipt.preview.enabled !== undefined)
                draft.enabled = receipt.preview.enabled;
        }
        draft.lastReceipt = { role: receipt.role, operationId: receipt.operationId, status: receipt.status };
        delete draft.pending;
        this.profile = undefined;
        let storageError: unknown;
        try {
            this.retain();
        }
        catch (error) {
            storageError = error;
        }
        setUiText(this.message, receipt.status === "committed" ? uiMessage("hostedLibrary.operationCommittedRereadBasis") : uiMessage("hostedLibrary.operationEndedRereadBasis"));
        this.render();
        if (receipt.status === "committed")
            await this.changed(receipt.role);
        if (storageError)
            throw new UiError(displayMessage("hostedLibrary.receiptCacheUpdateFailed", { p0: uiDisplayParameter(receipt.status), p1: uiDisplayParameter(pending.request.operationId) }));
    }
    private render() {
        const focused = document.activeElement === this.source, selection = focused ? [this.source.selectionStart, this.source.selectionEnd] : undefined;
        disposeUiTree(this.editor);
        this.editor.replaceChildren();
        this.inputs = [];
        this.comparison = undefined;
        const draft = this.draft, profile = this.profile;
        if (this.unreadable !== undefined) {
            this.editor.append(ui("p", "hint", uiMessage("hostedLibrary.recoveryUnparseable")), ui("pre", "", this.unreadable), action(uiMessage("hostedLibrary.discardCorruptRecovery"), () => this.run(() => { sessionStorage.removeItem(this.key()); this.unreadable = undefined; this.draft = undefined; this.render(); })));
            this.renderActions();
            return;
        }
        if (!draft) {
            this.renderActions();
            return;
        }
        if (profile) {
            this.editor.append(ui("p", "hint", profile.installed ? uiMessage("hostedLibrary.configurationInstalled") : uiMessage("hostedLibrary.roleNotInstalledExplicitCreate")));
            for (const warning of profile.warnings ?? [])
                this.editor.append(ui("p", "hint", descriptionUiText(warning)));
        }
        if (this.role === "style") {
            const enabledLabel = ui("label", "", uiMessage("hostedLibrary.automaticallyApplyDefaultStyle"));
            this.enabled.type = "checkbox";
            this.enabled.checked = draft.enabled ?? false;
            this.enabled.disabled = !this.writable();
            setUiAttr(this.enabled, "aria-label", uiMessage("hostedLibrary.enableHostedStyle"));
            this.enabled.onchange = () => { draft.enabled = this.enabled.checked; this.edited(); };
            enabledLabel.append(this.enabled);
            this.editor.append(enabledLabel);
            const controls = ui("section");
            setUiAttr(controls, "aria-label", uiMessage("hostedLibrary.nativeStyleControls"));
            for (const control of profile?.controls ?? []) {
                const raw = draft.changes[control.key], value: NativeStyleValue = raw === undefined ? control.value : raw;
                const label = ui("label", "", control.label), input = nativeStyleControlInput(control, value);
                setUiAttr(input, "aria-label", control.label);
                if (raw !== undefined)
                    input.value = raw;
                if (input instanceof HTMLInputElement)
                    this.authorSession(draft)?.bind(input, `style:${control.key}`);
                input.disabled = !this.writable() || !control.editable || draft.source !== draft.baseSource;
                input.oninput = () => { draft.changes[control.key] = input.value; this.edited(); };
                input.onchange = () => { draft.changes[control.key] = input.value; this.edited(); };
                label.append(input);
                if (control.reason)
                    label.append(ui("span", "hint", nativeStyleControlReason(control.reason)));
                controls.append(label);
                this.inputs.push({ control, input });
            }
            this.editor.append(controls, ui("p", "hint", uiMessage("hostedLibrary.styleControlsSourceBoundary")));
            for (const [key, value] of Object.entries(draft.changes))
                if (!profile?.controls?.some(control => control.key === key && control.editable))
                    this.editor.append(ui("p", "hint", displayMessage("hostedLibrary.unmappedOriginalControl", { p0: uiDisplayParameter(key), p1: uiDisplayParameter(String(value)) })), action(displayMessage("hostedLibrary.discardOriginalControlInput", { p0: uiDisplayParameter(key) }), () => { delete draft.changes[key]; this.edited(); this.render(); }));
            if (Object.keys(draft.changes).length)
                this.editor.append(action(uiMessage("hostedLibrary.discardControlInputUseSource"), () => { draft.changes = {}; this.edited(); this.render(); }));
        }
        if (this.role === "settings") {
            this.editor.append(ui("p", "hint", this.catalog.reorderNodes ? uiMessage("hostedLibrary.siblingReorderGranted") : uiMessage("hostedLibrary.siblingReorderNotGranted")));
            this.editor.append(this.orderHost);
            this.renderOrder();
            const processing = ui("section");
            setUiAttr(processing, "aria-label", uiMessage("hostedLibrary.sharedProcessingSuggestions"));
            processing.append(ui("h3", "", uiMessage("hostedLibrary.sharedProcessingAndVocabulary")), ui("p", "hint", uiMessage("hostedLibrary.sharedProcessingDescription")));
            try {
                const settings = JSON.parse(draft.source) as Record<string, unknown>;
                if (!settings || Array.isArray(settings) || settings.version !== 1)
                    throw new UiError(uiMessage("hostedLibrary.settingsVersionUnsupported"));
                if (settings.processing === undefined) {
                    const create = action(uiMessage("hostedLibrary.addSharedProcessingSuggestion"), () => { settings.processing = defaultProcessingDocument(); draft.source = JSON.stringify(settings, null, 2); this.edited(); this.render(); });
                    create.disabled = !this.writable();
                    processing.append(create);
                }
                else {
                    const document = decodeProcessingDocument(settings.processing);
                    const label = ui("label", "", uiMessage("hostedLibrary.defaultProcessingLanguages")), input = ui("input", "knowledge-input");
                    input.value = document.defaults.languages.map(processingLanguageTag).join(", ");
                    this.authorSession(draft)?.bind(input, "processing-languages");
                    input.disabled = !this.writable();
                    input.onchange = () => {
                        try {
                            document.defaults.languages = parseProcessingLanguages(input.value);
                            settings.processing = document;
                            draft.source = JSON.stringify(settings, null, 2);
                            this.edited();
                            this.render();
                        }
                        catch (error) {
                            setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error));
                        }
                    };
                    label.append(input);
                    processing.append(label, ui("p", "hint", displayMessage("hostedLibrary.processingRuleSummary", { p0: document.rules.length, p1: document.scopedAllowlist.length })));
                }
            }
            catch {
                processing.append(ui("p", "hint", uiMessage("hostedLibrary.processingSettingsCannotParseSafely")));
            }
            this.editor.append(processing);
        }
        const sourceLabel = ui("label", "", this.role === "style" ? uiMessage("hostedLibrary.fullNativeStyleSource") : uiMessage("hostedLibrary.fullJsonConfigurationSource"));
        this.source = ui("textarea");
        this.source.rows = 14;
        this.source.spellcheck = false;
        setUiAttr(this.source, "aria-label", labelSource);
        if (this.source.value !== draft.source)
            this.source.value = draft.source;
        this.authorSession(draft)?.bind(this.source, "source");
        this.source.readOnly = !this.writable();
        this.source.oninput = () => { draft.source = this.source.value; this.edited(); };
        sourceLabel.append(this.source);
        this.editor.append(sourceLabel);
        if (this.needsComparison()) {
            this.editor.append(ui("h3", "", uiMessage("hostedLibrary.configurationBasisChangedBothRetained")), ui("p", "", uiMessage("hostedLibrary.originalReadBasis")), ui("pre", "", draft.baseSource), ui("p", "", uiMessage("hostedLibrary.currentServerConfiguration")), ui("pre", "", profile!.source));
            const compare = action(uiMessage("hostedLibrary.confirmCurrentSourceAsNewBasis"), () => this.run(() => {
                if (draft.pending)
                    throw new UiError(uiMessage("hostedLibrary.checkOriginalBeforeNewBasis"));
                draft.baseSource = profile!.source;
                draft.baseEnabled = profile!.enabled;
                this.edited();
                this.render();
            }));
            this.comparison = compare;
            this.editor.append(compare);
        }
        this.renderActions();
        if (selection) {
            this.source.focus();
            this.source.setSelectionRange(selection[0], selection[1]);
        }
    }
    private renderOrder() {
        disposeUiTree(this.orderHost);
        this.orderHost.replaceChildren();
        const draft = this.draft, profile = this.profile;
        if (this.role !== "settings" || !this.catalog.reorderNodes || !draft || !profile)
            return;
        setUiAttr(this.orderHost, "aria-label", uiMessage("hostedLibrary.hostedAuthorSiblingOrder"));
        this.orderHost.append(ui("h3", "", uiMessage("hostedLibrary.authorSiblingOrder")), ui("p", "hint", uiMessage("hostedLibrary.authorOrderDescription")));
        const parents = this.catalog.orderParents ?? [], picker = ui("select");
        setUiAttr(picker, "aria-label", uiMessage("hostedLibrary.authorOrderParent"));
        for (const parent of parents) {
            const option = ui("option", "", `${parent.title} · ${parent.reference}`);
            option.value = parent.reference;
            picker.append(option);
        }
        if (!parents.some(parent => parent.reference === this.orderParent))
            this.orderParent = parents[0]?.reference ?? "";
        picker.value = this.orderParent;
        picker.disabled = this.busy || !!draft.pending;
        picker.onchange = () => { this.orderParent = picker.value; this.order = undefined; this.renderOrder(); };
        const read = action(uiMessage("hostedLibrary.readCompleteAuthorChildren"), () => this.run(async () => {
            const parent = this.orderParent, epoch = this.epoch, basis = profile.basis;
            const result = await this.request<HostedOrder>("library/order/read", { parentRef: parent });
            if (epoch !== this.epoch || this.profile?.basis !== basis || this.draft !== draft || this.orderParent !== parent)
                return;
            if (result.parentRef !== parent || !Array.isArray(result.members) || !Array.isArray(result.orderedRefs) || typeof result.complete !== "boolean" || result.settingsSourceHash !== await nodeSourceHash(profile.source))
                throw new UiError(uiMessage("hostedLibrary.childrenOrSettingsBasisChanged"));
            this.order = result;
            this.renderOrder();
        }));
        read.disabled = this.busy || !!draft.pending || !this.orderParent;
        this.orderHost.append(picker, read);
        if (!parents.length)
            this.orderHost.append(ui("p", "hint", uiMessage("hostedLibrary.noAuthorizedNavigationParent")));
        const order = this.order;
        if (!order || order.parentRef !== this.orderParent)
            return;
        if (!order.complete) {
            this.orderHost.append(ui("p", "hint", uiMessage("hostedLibrary.childrenIncompleteCannotReorder")));
            return;
        }
        let settings: Record<string, unknown>;
        try {
            settings = JSON.parse(draft.source);
            if (!settings || Array.isArray(settings) || settings.version !== 1)
                throw Error();
        }
        catch {
            this.orderHost.append(ui("p", "hint", uiMessage("hostedLibrary.jsonIncomplete")));
            return;
        }
        const orders = settings.navigationOrders as Record<string, unknown> | undefined, stored = orders?.[order.parentRef];
        if (stored !== undefined && (!Array.isArray(stored) || stored.some(ref => typeof ref !== "string"))) {
            this.orderHost.append(ui("p", "hint", uiMessage("hostedLibrary.parentOrderNotReferenceArray")));
            return;
        }
        const actual = new Set(order.orderedRefs), refs = (stored as string[] | undefined)?.filter(ref => actual.has(ref)) ?? [...order.orderedRefs];
        for (const ref of order.orderedRefs)
            if (!refs.includes(ref))
                refs.push(ref);
        for (const [ordinal, ref] of refs.entries()) {
            const row = ui("div", "hosted-order-row"), name = order.members.find(member => member.targetRef === ref)?.name ?? uiMessage("hostedLibrary.originalStableObject");
            row.append(ui("span", "", () => `${uiTextValue(ordinal + 1)}. ${uiTextValue(name)} · ${uiTextValue(ref)}`));
            for (const [direction, label] of [[-1, uiMessage("queryShortcut.moveUp")], [1, uiMessage("nodeContents.moveDown")]] as const) {
                const move = action(label, () => {
                    if (this.draft !== draft || this.order !== order || draft.pending || this.busy)
                        return;
                    const latest = JSON.parse(draft.source) as Record<string, unknown>;
                    if (!latest || Array.isArray(latest) || latest.version !== 1)
                        throw new UiError(uiMessage("hostedLibrary.jsonChangedInputNotOverwritten"));
                    if (latest.navigationOrders === undefined)
                        latest.navigationOrders = {};
                    if (!latest.navigationOrders || typeof latest.navigationOrders !== "object" || Array.isArray(latest.navigationOrders))
                        throw new UiError(uiMessage("hostedLibrary.navigationOrdersInvalid"));
                    (latest.navigationOrders as Record<string, unknown>)[order.parentRef] = movedSibling(refs, ref, direction);
                    draft.source = JSON.stringify(latest, null, 2);
                    this.source.value = draft.source;
                    this.edited(true);
                    this.renderOrder();
                });
                setUiAttr(move, "aria-label", displayMessage("hostedLibrary.authorOrderAction", { p0: uiDisplayParameter(name), p1: uiDisplayParameter(label) }));
                move.disabled = this.busy || !!draft.pending || direction === -1 && ordinal === 0 || direction === 1 && ordinal === refs.length - 1;
                row.append(move);
            }
            this.orderHost.append(row);
        }
    }
    private renderActions() {
        if (this.comparison)
            this.comparison.disabled = this.busy || !!this.draft?.pending;
        disposeUiTree(this.actions);
        this.actions.replaceChildren();
        if (!this.role)
            return;
        const draft = this.draft, pending = draft?.pending;
        const read = action(uiMessage("hostedLibrary.readConfiguration"), () => this.run(() => this.read()));
        read.disabled = this.busy || this.unreadable !== undefined;
        this.actions.append(read);
        const prepare = action(uiMessage("hostedLibrary.prepareConfigurationChanges"), () => this.run(() => this.prepare()));
        prepare.disabled = this.busy || !this.writable() || !draft || !this.profile || !!pending || this.needsComparison();
        this.actions.append(prepare);
        if (draft?.lastReceipt)
            this.actions.append(ui("p", "hint", displayMessage("hostedLibrary.originalConfigurationReceipt", { p0: uiDisplayParameter(draft.lastReceipt.operationId), p1: uiDisplayParameter(draft.lastReceipt.status), p2: uiDisplayParameter(draft.lastReceipt.status === "committed" ? uiMessage("hostedLibrary.committedFactNotCurrentBasisSuffix") : uiMessage("hostedLibrary.notTreatedAsSavedSuffix")) })));
        if (!pending)
            return;
        const card = ui("section");
        setUiAttr(card, "aria-label", uiMessage("hostedLibrary.checkOriginalConfigurationOperation"));
        card.append(ui("h3", "", uiMessage("hostedLibrary.checkOriginalConfigurationOperation")), ui("p", "", `${pending.request.role} · ${pending.request.operationId} · ${pending.receipt?.status ?? "unknown"}`));
        const preview = this.verifiedOperation === pending.request.operationId ? pending.receipt?.preview : undefined;
        if (preview?.summary) card.append(ui("p", "", preview.summary));
        for (const [label, value] of [[uiMessage("hostedLibrary.configurationBeforeOriginalPlan"), preview?.beforeSource], [uiMessage("hostedLibrary.configurationAfterOriginalPlan"), preview?.afterSource]] as const)
            if (typeof value === "string") {
                const area = ui("textarea");
                area.rows = 8;
                area.readOnly = true;
                area.value = value;
                setUiAttr(area, "aria-label", label);
                card.append(ui("p", "", label), area);
            }
        if (preview?.enabled !== undefined)
            card.append(ui("p", "hint", displayMessage("hostedLibrary.originalPlanLibraryStyle", { p0: uiDisplayParameter(preview.enabled ? uiMessage("hostedLibrary.styleEnabled") : uiMessage("hostedLibrary.styleDisabled")) })));
        if (draft && pending.revision !== draft.revision)
            card.append(ui("p", "hint", uiMessage("hostedLibrary.inputDiffersFromFrozenProposal")));
        const query = action(uiMessage("hostedLibrary.queryOriginalConfigurationOperation"), () => this.run(() => this.operation()));
        query.disabled = this.busy;
        const retry = action(uiMessage("hostedLibrary.retryOriginalPrepareRequest"), () => this.run(() => this.sendPrepare()));
        retry.disabled = this.busy || !this.writable() || !!pending.commit;
        const settle = action(uiMessage("hostedLibrary.endOriginalOperationKeepInput"), () => this.run(() => this.operation(true)));
        settle.disabled = this.busy || !this.writable();
        const commit = action(pending.commit ? uiMessage("hostedLibrary.retryOriginalCommitRequest") : uiMessage("hostedLibrary.confirmOriginalConfigurationPlan"), () => this.run(() => this.commit()));
        commit.disabled = this.busy || this.verifiedOperation !== pending.request.operationId || !draft || !libraryCanCommit(draft) || !this.writable() || this.needsComparison();
        card.append(query, retry, settle, commit);
        this.actions.append(card);
    }
}
