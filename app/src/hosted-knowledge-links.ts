import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./knowledge-view.css";
import { action, ui } from "./data-panel";
import { nodeSourceHash } from "./hosted-navigation-state";
import { hostedLinksPatch, requireHostedLinksBasis, sameHostedLinksContext, sameHostedLinksIdentity, type HostedLinksBasis, type HostedLinksContext, type HostedLinksIdentity, type HostedLinksEdge, type HostedLinksHit, type HostedLinksProfile, type HostedLinksProposal, type HostedLinksResolution } from "./hosted-knowledge-links-state";
import type { SourcePatch } from "./editor";
import { PersonalRelationRules, RelationshipDiagnosticsView, visibleCandidate, type RelationNode, type RelationIssue } from "./relationship-diagnostics";
import type { HostedLinksNode } from "./hosted-knowledge-links-state";
import { RelationshipGraphView } from "./relationship-graph-view";
import { HeadingStructureView } from "./heading-structure-view";
import type { StructureHeading } from "./heading-structure-state";
type Request = <T>(path: string, body?: unknown, signal?: AbortSignal) => Promise<T>;
export interface HostedKnowledgeLinksHost {
    context(): HostedLinksContext | null;
    identity(): HostedLinksIdentity | null;
    flush(): Promise<void>;
    apply(patches: SourcePatch[], basis: {
        sessionId: string;
        source: string;
        serial: number;
    }, event: string): Promise<void>;
    navigate(reference: string): Promise<void>;
    select(from: number, to: number): void;
    revealRendered?(heading: StructureHeading): boolean;
    authorizationChanged(message: UiText, identity: {
        actor: string;
        deployment: string;
        generation: number;
    }): void;
}
const states: Record<string, UiText> = { resolved: uiMessage("knowledge.targetValid"), unavailable: uiMessage("hostedLinks.targetCurrentlyUnavailable"), missing: uiMessage("hostedLinks.targetNotFoundInAuthorizedScope"), ambiguous: uiMessage("hostedLinks.targetOrFragmentAmbiguous"), fragmentMissing: uiMessage("hostedLinks.targetFragmentMissing"), invalid: uiMessage("hostedLinks.originalAddressInvalid"), external: uiMessage("hostedLinks.externalAddress"), inDocument: uiMessage("hostedLinks.inDocumentReference") };
/** No independent editor, persistence protocol or protected result cache.
 * Every button is keyboard accessible, including all overlapping candidates. */
export class HostedKnowledgeLinks {
    private epoch = 0;
    private busy = false;
    private stale = false;
    private composing = false;
    private deferredPaint = false;
    private controller?: AbortController;
    private profile?: HostedLinksProfile;
    private captured?: HostedLinksContext;
    private hash?: string;
    private showMentions = false;
    private mentionDomain = "all";
    private exclusionScope = "note";
    private rules = new PersonalRelationRules();
    private diagnostics = this.diagnosticView();
    private graph = new RelationshipGraphView();
    private headings = new HeadingStructureView();
    private viewSession = "";
    private diagnosticView(): RelationshipDiagnosticsView {
        return new RelationshipDiagnosticsView(this.rules, async (node) => {
            const { context, epoch } = this.current();
            if (await this.audience(context, epoch))
                await this.owner.navigate(node.reference);
        }, async (issue) => {
            const { context, epoch } = this.current();
            if (!await this.audience(context, epoch))
                return;
            await this.owner.navigate(issue.source.reference);
            if (issue.field) {
                setUiText(this.message, uiMessage("hostedLinks.propertyOwnerOpenedNoExactSource"));
                return;
            }
            const opened = this.owner.context();
            if (!opened || !issue.sourceHash || await nodeSourceHash(opened.source) !== issue.sourceHash) {
                setUiText(this.message, uiMessage("hostedLinks.sourceNoteOpenedBodyChanged"));
                return;
            }
            if (issue.from !== undefined && issue.to !== undefined)
                this.owner.select(issue.from, issue.to);
        }, error => { setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error)); });
    }
    private relationNode(node: HostedLinksNode): RelationNode { return { key: `${node.nodeRef.workspaceId}/${node.nodeRef.nodeId}`, title: node.title, reference: node.reference }; }
    private readonly content = ui("div");
    private readonly message = ui("p", "hint");
    private readonly refresh = action(uiMessage("hostedLinks.readLinksMentions"), () => this.run(() => this.read()));
    private readonly stop = action(uiMessage("hostedLinks.stopWaitingForScan"), () => { this.epoch++; this.controller?.abort(); this.controller = undefined; this.busy = false; this.profile = undefined; this.content.replaceChildren(); this.refresh.disabled = false; this.stop.disabled = true; setUiText(this.message, uiMessage("hostedLinks.scanWaitStopped")); });
    constructor(private readonly host: HTMLElement, private readonly request: Request, private readonly owner: HostedKnowledgeLinksHost) {
        this.clear();
        host.addEventListener("compositionstart", () => { this.composing = true; });
        host.addEventListener("compositionend", () => {
            this.composing = false;
            if (this.deferredPaint) {
                this.deferredPaint = false;
                this.paint();
            }
        });
    }
    clear(): void { this.graph.clear(); this.headings.clear(); this.rules.closePreviews(); this.epoch++; this.controller?.abort(); this.controller = undefined; this.profile = undefined; this.captured = undefined; this.hash = undefined; this.busy = false; this.showMentions = false; this.rules = new PersonalRelationRules(); this.diagnostics = this.diagnosticView(); this.content.replaceChildren(); this.host.replaceChildren(); this.host.hidden = true; }
    connect(): void { this.clear(); this.documentChanged(); }
    async showStructure(kind: "graph" | "headings"): Promise<void> {
        if (!this.profile)
            await this.run(() => this.read());
        const target = this.content.querySelector<HTMLDetailsElement>(`details[data-knowledge-view="${kind}"]`);
        if (target) {
            target.open = true;
            target.scrollIntoView({ block: "start", behavior: "instant" });
            target.querySelector<HTMLElement>("summary")?.focus();
        }
    }
    documentChanged(): void {
        if (this.viewSession !== (this.owner.context()?.sessionId ?? "")) {
            this.viewSession = this.owner.context()?.sessionId ?? "";
            this.graph.clear();
            this.headings.clear();
        }
        this.rules.closePreviews();
        this.epoch++;
        this.controller?.abort();
        this.controller = undefined;
        this.profile = undefined;
        this.captured = undefined;
        this.hash = undefined;
        this.busy = false;
        this.content.replaceChildren();
        this.host.hidden = !this.owner.context();
        if (!this.host.hidden) {
            setUiText(this.message, uiMessage("hostedLinks.authorizedScanScopeDescription"));
            this.host.replaceChildren(ui("h2", "", uiMessage("hostedLinks.linksAndPotentialMentions")), this.refresh, this.stop, this.message, this.content);
        }
        this.stop.disabled = true;
        this.refresh.disabled = false;
    }
    environmentChanged(): void { this.documentChanged(); }
    private async run(work: () => Promise<void>): Promise<void> {
        if (this.busy)
            return;
        this.busy = true;
        const epoch = this.epoch;
        this.refresh.disabled = true;
        this.stop.disabled = false;
        try {
            await work();
        }
        catch (error) {
            if (epoch === this.epoch) {
                const http = (error as {
                    http?: number;
                })?.http;
                let retained = false;
                if (http && (http >= 500 || http === 429) && this.captured && this.profile) {
                    try {
                        retained = await this.audience(this.captured, epoch);
                    }
                    catch { /* Cannot retain protected results without current audience. */ }
                }
                if (epoch !== this.epoch)
                    return;
                if (retained)
                    this.stale = true;
                else {
                    this.profile = undefined;
                    this.content.replaceChildren();
                }
                setUiText(this.message, () => `${uiTextValue(retained ? uiMessage("hostedLinks.scanFailedRetainedResultStale") : "")}${uiTextValue(error instanceof Error ? errorUiText(error) : uiMessage("hostedLinks.readIncompleteDraftLinksRetained"))}`);
            }
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.refresh.disabled = false;
                this.stop.disabled = true;
                if (this.profile)
                    this.paint();
            }
        }
    }
    private accepts(context: HostedLinksContext, epoch: number): boolean { return epoch === this.epoch && sameHostedLinksContext(context, this.owner.context()); }
    private async audience(context: HostedLinksContext, epoch: number): Promise<boolean> {
        const current = await this.request<{
            actor: string;
            deployment: string;
            generation: number;
        }>("me");
        if (!sameHostedLinksIdentity(context, this.owner.identity()))
            return false;
        if (current.actor !== context.actor || current.deployment !== context.deployment || current.generation !== context.generation) {
            this.clear();
            this.owner.authorizationChanged(uiMessage("hostedLinks.authorizationChangedReconnect"), current);
            return false;
        }
        return this.accepts(context, epoch);
    }
    private async read(): Promise<void> {
        await this.owner.flush();
        const context = this.owner.context();
        if (!context)
            throw new UiError(uiMessage("hostedLinks.openAuthorizedNoteFirst"));
        const epoch = this.epoch, hash = await nodeSourceHash(context.source);
        if (!this.accepts(context, epoch))
            return;
        const controller = new AbortController();
        this.controller = controller;
        const result = await this.request<HostedLinksProfile>("knowledge-links/read", { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, controller.signal);
        if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch))
            return;
        requireHostedLinksBasis(result.basis, context, hash);
        if (result.propertyReferences?.scope !== "explicitAuthorizedFields" || result.mentions?.scope !== "nativeTitlesAndAuthorizedNameFields")
            throw new UiError(uiMessage("hostedLinks.fieldOrDictionaryScopeUnconfirmed"));
        if (result.scope !== "configuredAuthorizedManagedSources" || !["complete", "partial"].includes(result.links?.coverage) || !["complete", "partial"].includes(result.mentions?.coverage) || !Array.isArray(result.links.outgoing) || !Array.isArray(result.links.incomingId) || !Array.isArray(result.links.incomingPath) || !Array.isArray(result.mentions.fragments) || !Array.isArray(result.renderedLinks) || !Array.isArray(result.propertyReferences?.items) || result.fileConversion !== false)
            throw new UiError(uiMessage("hostedLinks.linkScopeOrFormatUnconfirmed"));
        this.profile = result;
        this.captured = context;
        this.hash = hash;
        this.stale = false;
        setUiText(this.message, result.links.coverage === "complete" ? uiMessage("hostedLinks.linksReadForAuthorizedScope") : uiMessage("hostedLinks.scanIncompleteConfirmedResultsOnly"));
    }
    private current(): {
        context: HostedLinksContext;
        profile: HostedLinksProfile;
        hash: string;
        epoch: number;
    } {
        const context = this.captured, profile = this.profile, hash = this.hash;
        if (this.stale || !context || !profile || !hash || !this.accepts(context, this.epoch))
            throw new UiError(uiMessage("hostedLinks.draftOrDisplayBasisChanged"));
        return { context, profile, hash, epoch: this.epoch };
    }
    private async convert(intent: unknown): Promise<void> {
        const original = this.current();
        await this.owner.flush();
        if (!this.accepts(original.context, original.epoch))
            throw new UiError(uiMessage("hostedLinks.inputChangedBeforeConversion"));
        const { context, profile, hash, epoch } = original;
        if (!context.writable || context.pending || context.compare || !profile.writable)
            throw new UiError(uiMessage("hostedLinks.conversionRequiresWritableCheckedDraft"));
        const result = await this.request<HostedLinksProposal>("knowledge-links/convert", { basis: { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, snapshot: profile.basis.snapshot, intent });
        if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch))
            return;
        const patch = hostedLinksPatch(result, context, hash, profile.basis.snapshot);
        // The host's existing shared editor independently checks composition and
        // exact source/serial before one native Undo transaction.
        await this.owner.apply([patch], { sessionId: context.sessionId, source: context.source, serial: context.serial }, "input.knowledge.link.proposal");
        this.documentChanged();
        setUiText(this.message, uiMessage("hostedLinks.conversionAddedToDraftUndo"));
    }
    private async copy(edge: HostedLinksEdge, form: string, full: boolean): Promise<void> {
        const original = this.current();
        await this.owner.flush();
        if (!this.accepts(original.context, original.epoch))
            throw new UiError(uiMessage("hostedLinks.draftChangedBeforeCopy"));
        const { context, profile, hash, epoch } = original;
        const result = await this.request<{
            basis: HostedLinksBasis;
            address: string;
            fullLink: string;
            literal: boolean;
        }>("knowledge-links/copy", { basis: { sessionId: context.sessionId, source: context.source, sourceHash: hash, serial: context.serial }, snapshot: profile.basis.snapshot, occurrence: { from: edge.occurrence.from, to: edge.occurrence.to, target: edge.occurrence.target }, form });
        if (!sameHostedLinksIdentity(context, this.owner.identity()) || !await this.audience(context, epoch))
            return;
        requireHostedLinksBasis(result.basis, context, hash, profile.basis.snapshot);
        if (typeof result.address !== "string" || typeof result.fullLink !== "string" || form === "original" && (!result.literal || result.address !== edge.occurrence.target || result.fullLink !== edge.raw))
            throw new UiError(uiMessage("hostedLinks.copyReceiptOriginalLinkMismatch"));
        await navigator.clipboard.writeText(full ? result.fullLink : result.address);
        if (this.accepts(context, epoch))
            setUiText(this.message, full ? uiMessage("hostedLinks.fullNativeLinkCopied") : uiMessage("hostedLinks.targetAddressCopied"));
    }
    private destination(parent: HTMLElement, resolution: HostedLinksResolution): void {
        parent.append(ui("span", "hint", states[resolution.status] ?? uiMessage("hostedLinks.targetCurrentlyUnavailable")));
        if (resolution.canOpen && resolution.reference)
            parent.append(action(uiMessage("hostedLinks.openAuthorizedTarget"), () => this.run(async () => {
                const { context, epoch } = this.current();
                if (await this.audience(context, epoch))
                    await this.owner.navigate(resolution.reference!);
            })));
    }
    private edge(edge: HostedLinksEdge, outgoing: boolean): HTMLElement {
        const row = ui("div", "query-result-row");
        row.append(ui("p", "", outgoing ? edge.occurrence.target : `${edge.source.title} · ${edge.occurrence.target}`));
        this.destination(row, edge.resolution);
        if (!outgoing) {
            const open = action(uiMessage("hostedLinks.openSourceNote"), () => this.run(async () => {
                const { context, epoch } = this.current();
                if (await this.audience(context, epoch))
                    await this.owner.navigate(edge.source.reference);
            }));
            open.disabled = !edge.source.canOpen;
            row.append(open);
            return row;
        }
        row.append(action(uiMessage("hostedLinks.selectSourceOccurrence"), () => this.run(async () => { this.current(); this.owner.select(edge.occurrence.utf16From, edge.occurrence.utf16To); })));
        const forms = ui("select");
        setUiAttr(forms, "aria-label", displayMessage("hostedLinks.linkAddressForm", { p0: uiDisplayParameter(edge.occurrence.target) }));
        for (const [value, label] of [["original", uiMessage("hostedLinks.retainOriginal")], ["id", uiMessage("knowledge.formStableId")], ["relative", uiMessage("knowledge.formSourceRelative")], ["workspace", uiMessage("knowledge.formLibraryPath")]] as const) {
            const option = ui("option", "", label);
            option.value = value;
            forms.append(option);
        }
        const convert = action(uiMessage("hostedLinks.convertThisLink"), () => this.run(() => this.convert({ kind: "link", occurrence: { from: edge.occurrence.from, to: edge.occurrence.to, target: edge.occurrence.target }, form: forms.value })));
        const update = (): void => { convert.disabled = this.busy || !this.profile?.writable || !this.owner.context()?.writable || !!this.owner.context()?.pending || !!this.owner.context()?.compare || !edge.editable || forms.value === "original" || edge.resolution.status !== "resolved"; };
        forms.onchange = update;
        update();
        row.append(forms, convert, action(uiMessage("hostedLinks.copyTargetAddress"), () => this.run(() => this.copy(edge, forms.value, false))), action(uiMessage("knowledge.copyCompleteLink"), () => this.run(() => this.copy(edge, forms.value, true))));
        return row;
    }
    private mention(fragment: string, hit: HostedLinksHit): HTMLElement {
        const row = ui("div", "query-result-row");
        row.append(ui("p", "", displayMessage("hostedLinks.mentionTextRange", { p0: uiDisplayParameter(hit.terms.join(" / ")), p1: hit.startScalar, p2: hit.endScalar })));
        for (const candidate of hit.candidates ?? hit.targets.map(target => ({ target, term: hit.terms[0], provenance: "title", fragment: undefined }))) {
            const target = candidate.target;
            if ((this.mentionDomain === "fragments" && !candidate.fragment || this.mentionDomain === "notes" && candidate.fragment) || !visibleCandidate(this.rules.rules, this.owner.context()?.resource ?? "", candidate.term, this.relationNode(target).key))
                continue;
            const choice = ui("div"), open = action(uiMessage("hostedLinks.openCandidate"), () => this.run(async () => {
                const { context, epoch } = this.current();
                if (await this.audience(context, epoch))
                    await this.owner.navigate(`${target.reference}${candidate.fragment ? `#${encodeURIComponent(candidate.fragment)}` : ""}`);
            }));
            open.disabled = !target.canOpen;
            choice.append(ui("span", "", () => `${uiTextValue(target.title)}${uiTextValue(candidate.fragment ? ` #${candidate.fragment}` : "")} · ${uiTextValue(({ title: uiMessage("knowledge.noteTitle"), nameField: uiMessage("knowledge.nameField"), heading: uiMessage("knowledge.headingText"), stableLabel: uiMessage("knowledge.stableLabel") })[candidate.provenance] ?? candidate.provenance)}`), open);
            const convert = action(displayMessage("hostedLinks.convertRangeToLink", { p0: uiDisplayParameter(target.title), p1: uiDisplayParameter(candidate.fragment ? ` #${candidate.fragment}` : "") }), () => this.run(() => this.convert({ kind: "mention", fragmentId: fragment, startScalar: hit.startScalar, endScalar: hit.endScalar, target: target.nodeRef, fragment: candidate.fragment })));
            const context = this.owner.context();
            convert.disabled = this.busy || !hit.writable || !this.profile?.writable || !context?.writable || !!context.pending || !!context.compare;
            const source = this.exclusionScope === "note" ? this.owner.context()?.resource : undefined;
            choice.append(convert, action(uiMessage("knowledge.excludeTarget"), () => this.rules.preview({ kind: "target", value: this.relationNode(target).key, source }, displayMessage("hostedLinks.excludeTargetDescription", { p0: uiDisplayParameter(target.title), p1: uiDisplayParameter(this.relationNode(target).key), p2: uiDisplayParameter(source ? uiMessage("knowledge.scopeThisNote") : uiMessage("hostedLinks.pageAuthorizationScope")) }), this.profile?.mentions.fragments.flatMap(f => f.hits).filter(h => h.targets.some(t => t.resource === target.resource)).length ?? 0, () => this.paint())), action(uiMessage("knowledge.ignoreTerm"), () => this.rules.preview({ kind: "term", value: candidate.term, source }, displayMessage("hostedLinks.ignoreTermDescription", { p0: uiDisplayParameter(candidate.term), p1: uiDisplayParameter(source ? uiMessage("knowledge.scopeThisNote") : uiMessage("hostedLinks.pageAuthorizationScope")) }), this.profile?.mentions.fragments.flatMap(f => f.hits).filter(h => h.terms.includes(candidate.term)).length ?? 0, () => this.paint())));
            row.append(choice);
        }
        if (!hit.writable)
            row.append(ui("p", "hint", uiMessage("hostedLinks.mentionNoUniqueWritableRange")));
        return row;
    }
    private paint(): void {
        const profile = this.profile;
        if (!profile)
            return;
        if (this.composing) {
            this.deferredPaint = true;
            return;
        }
        const active = document.activeElement;
        const focus = active instanceof HTMLInputElement && this.content.contains(active) ? { label: active.getAttribute("aria-label"), start: active.selectionStart, end: active.selectionEnd } : null;
        this.content.replaceChildren();
        const graphSection = ui("details"), graphTitle = ui("summary", "", uiMessage("hostedLinks.relationshipGraph"));
        graphSection.dataset.knowledgeView = "graph";
        graphSection.append(graphTitle);
        const root = profile.graph?.root ?? "";
        this.graph.render(graphSection, profile.graph, root, {
            open: async (node) => {
                const { context, epoch } = this.current();
                if (await this.audience(context, epoch))
                    await this.owner.navigate(node.reference);
            },
            source: async (edge, node) => {
                const { context, epoch } = this.current();
                if (!await this.audience(context, epoch))
                    return;
                await this.owner.navigate(node.reference);
                const opened = this.owner.context();
                if (!opened || await nodeSourceHash(opened.source) !== edge.sourceHash) {
                    setUiText(this.message, uiMessage("knowledge.authorSourceOpenedBodyChanged"));
                    return;
                }
                if (edge.range)
                    this.owner.select(edge.range.utf16From, edge.range.utf16To);
                else
                    setUiText(this.message, displayMessage("hostedLinks.authorSourceOpenedEditAuthorizedField", { p0: uiDisplayParameter(edge.fieldId ?? "") }));
            }, error: error => { setUiText(this.message, error instanceof Error ? errorUiText(error) : String(error)); },
        });
        this.content.append(graphSection);
        const headingSection = ui("details");
        headingSection.dataset.knowledgeView = "headings";
        headingSection.append(ui("summary", "", uiMessage("heading.readOnlyStructure")));
        this.headings.render(headingSection, profile.headingStructure, heading => {
            void this.run(async () => {
                const { context, epoch } = this.current();
                if (!await this.audience(context, epoch))
                    return;
                if (heading.range)
                    this.owner.select(heading.range.utf16From, heading.range.utf16To);
                else if (!this.owner.revealRendered?.(heading))
                    setUiText(this.message, uiMessage("hostedLinks.generatedHeadingAddressUnavailable"));
            });
        });
        this.content.append(headingSection);
        const d = profile.diagnostics;
        this.diagnostics.render(this.content, d && { ...d, issues: d.issues.map<RelationIssue>(i => ({ source: this.relationNode(i.source), target: i.resolution.target ? this.relationNode(i.resolution.target) : undefined, address: i.occurrence.target, status: i.resolution.status, from: i.occurrence.utf16From, to: i.occurrence.utf16To, sourceHash: i.sourceHash })).concat((d.propertyIssues ?? []).map(p => ({ source: this.relationNode(p.source), target: p.resolution.target ? this.relationNode(p.resolution.target) : undefined, address: p.address, status: p.resolution.status, from: undefined, to: undefined, sourceHash: undefined, field: [p.fieldId, ...p.itemPath].join(" / ") }))), nodes: d.nodes.map(n => ({ ...n, node: this.relationNode(n.node) })) });
        for (const [label, edges, outgoing] of [[uiMessage("knowledge.outgoingLinks"), profile.links.outgoing, true], [uiMessage("hostedLinks.stableIdBacklinks"), profile.links.incomingId, false], [uiMessage("hostedLinks.currentPathIncomingLinks"), profile.links.incomingPath, false]] as const) {
            const section = ui("section");
            section.append(ui("h3", "", label));
            if (!edges.length)
                section.append(ui("p", "hint", profile.links.coverage === "partial" ? uiMessage("hostedLinks.partialScanNoConfirmedResults") : uiMessage("hostedLinks.authorizedScopeNoResults")));
            for (const edge of edges)
                section.append(this.edge(edge, outgoing));
            this.content.append(section);
        }
        const rendered = ui("section");
        rendered.append(ui("h3", "", uiMessage("hostedLinks.compiledHtmlLinks")));
        for (const link of profile.renderedLinks) {
            const row = ui("div");
            row.append(ui("p", "", link.destination));
            this.destination(row, link.resolution);
            row.append(ui("p", "hint", uiMessage("hostedLinks.compiledOccurrenceReadOnly")));
            rendered.append(row);
        }
        this.content.append(rendered);
        const properties = ui("section");
        properties.append(ui("h3", "", uiMessage("hostedLinks.propertyReferences")), ui("p", "hint", uiMessage("hostedLinks.authorizedExactFieldReferencesOnly")));
        if (profile.propertyReferences.coverage === "partial")
            properties.append(ui("p", "hint", uiMessage("hostedLinks.propertyReferenceResolutionIncomplete")));
        for (const item of profile.propertyReferences.items) {
            const row = ui("div");
            row.append(ui("p", "", `${item.source.title} · ${item.fieldId}`));
            this.destination(row, item.resolution);
            properties.append(row);
        }
        this.content.append(properties);
        const mentions = ui("section"), show = ui("input");
        show.type = "checkbox";
        show.checked = this.showMentions;
        const label = ui("label");
        label.append(show, uiTextNode(uiMessage("hostedLinks.showPotentialMentions")));
        show.onchange = () => { this.showMentions = show.checked; this.paint(); };
        mentions.append(ui("h3", "", uiMessage("hostedLinks.potentialMentions")), label);
        mentions.append(ui("p", "hint", uiMessage("hostedLinks.potentialMentionDescription")));
        const domain = ui("select");
        setUiAttr(domain, "aria-label", uiMessage("knowledge.candidateScope"));
        for (const [v, t] of [["all", uiMessage("knowledge.titlesNamesStableFragments")], ["notes", uiMessage("hostedLinks.titlesNamesOnly")], ["fragments", uiMessage("knowledge.stableFragmentsOnly")]] as const) {
            const o = ui("option", "", t);
            o.value = v;
            domain.append(o);
        }
        domain.value = this.mentionDomain;
        domain.onchange = () => { this.mentionDomain = domain.value; this.paint(); };
        mentions.append(domain);
        const scope = ui("select");
        setUiAttr(scope, "aria-label", uiMessage("knowledge.exclusionRuleScope"));
        for (const [v, t] of [["note", uiMessage("knowledge.scopeThisNote")], ["page", uiMessage("hostedLinks.pageAuthorizationScope")]] as const) {
            const o = ui("option", "", t);
            o.value = v;
            scope.append(o);
        }
        scope.value = this.exclusionScope;
        scope.onchange = () => { this.exclusionScope = scope.value; };
        mentions.append(scope);
        this.rules.controls(mentions, () => this.paint());
        if (profile.mentions.coverage === "partial")
            mentions.append(ui("p", "hint", uiMessage("hostedLinks.textOrDictionaryIncomplete")));
        if (this.showMentions)
            for (const fragment of profile.mentions.fragments) {
                if (!fragment.hits.length)
                    continue;
                const group = ui("section");
                group.append(ui("p", "", fragment.text));
                if (fragment.coverage === "partial")
                    group.append(ui("p", "hint", uiMessage("hostedLinks.logicalFragmentPartiallyScanned")));
                for (const hit of fragment.hits)
                    group.append(this.mention(fragment.id, hit));
                mentions.append(group);
            }
        this.content.append(mentions);
        if (focus?.label) {
            const input = this.content.querySelector<HTMLInputElement>(`input[aria-label="${CSS.escape(focus.label)}"]`);
            input?.focus();
            if (input && focus.start !== null && focus.end !== null)
                input.setSelectionRange(focus.start, focus.end);
        }
    }
}
