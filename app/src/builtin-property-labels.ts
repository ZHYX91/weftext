/** Captions for the installed builtin business shapes, never author values or addresses. */
import bundled from "../../docs/design/examples/schema.json" with { type: "json" };
import type { DataSchema, SchemaField, Shape } from "./native-value-form";
import type { NativeControlIdentity } from "./native-control-identity";
import { message, type UiText } from "./ui-locale";

const fieldMessages: Readonly<Record<string, UiText>> = {
    "names": message("builtinProperty.field.names"),
    "phones": message("builtinProperty.field.phones"),
    "emails": message("builtinProperty.field.emails"),
    "addresses": message("builtinProperty.field.addresses"),
    "websites": message("builtinProperty.field.websites"),
    "aliases": message("builtinProperty.field.aliases"),
    "tags": message("builtinProperty.field.tags"),
    "accounts": message("builtinProperty.field.accounts"),
    "nationality_states": message("builtinProperty.field.nationalityStates"),
    "legal_sex_states": message("builtinProperty.field.legalSexStates"),
    "gender_identity_states": message("builtinProperty.field.genderIdentityStates"),
    "professions": message("builtinProperty.field.professions"),
    "engagements": message("builtinProperty.field.engagements"),
    "life_events": message("builtinProperty.field.lifeEvents"),
    "measurements": message("builtinProperty.field.measurements"),
    "avatar": message("builtinProperty.field.avatar"),
    "parents": message("builtinProperty.field.parents"),
    "spouses": message("builtinProperty.field.spouses"),
    "siblings": message("builtinProperty.field.siblings"),
    "guardians": message("builtinProperty.field.guardians"),
    "managers": message("builtinProperty.field.managers"),
    "mentors": message("builtinProperty.field.mentors"),
    "family_relations": message("builtinProperty.field.familyRelations"),
    "social_relations": message("builtinProperty.field.socialRelations"),
    "professional_relations": message("builtinProperty.field.professionalRelations"),
    "organization_states": message("builtinProperty.field.organizationStates"),
    "identifiers": message("builtinProperty.field.identifiers"),
    "classifications": message("builtinProperty.field.classifications"),
    "sites": message("builtinProperty.field.sites"),
    "parent": message("builtinProperty.field.parent"),
    "governs": message("builtinProperty.field.governs"),
    "owns": message("builtinProperty.field.owns"),
    "allied_with": message("builtinProperty.field.alliedWith"),
    "related_organizations": message("builtinProperty.field.relatedOrganizations"),
    "brand_of": message("builtinProperty.field.brandOf"),
    "business_guided_by": message("builtinProperty.field.businessGuidedBy"),
    "jointly_led_by": message("builtinProperty.field.jointlyLedBy"),
    "member_of": message("builtinProperty.field.memberOf"),
    "subsidiary_of": message("builtinProperty.field.subsidiaryOf"),
    "supervised_by": message("builtinProperty.field.supervisedBy"),
    "territorially_administered_by": message("builtinProperty.field.territoriallyAdministeredBy"),
    "status": message("builtinProperty.field.status"),
    "priority": message("builtinProperty.field.priority"),
    "start": message("builtinProperty.field.start"),
    "due": message("builtinProperty.field.due"),
    "completed_at": message("builtinProperty.field.completedAt"),
    "dependencies": message("builtinProperty.field.dependencies"),
    "project": message("builtinProperty.field.project"),
    "publication": message("builtinProperty.field.publication"),
    "period": message("builtinProperty.field.period"),
    "range": message("builtinProperty.field.range"),
    "event_status": message("builtinProperty.field.eventStatus"),
    "recurrence": message("builtinProperty.field.recurrence"),
    "participants": message("builtinProperty.field.participants"),
    "reminders": message("builtinProperty.field.reminders"),
    "reminder": message("builtinProperty.field.reminder")
};

const memberMessages: Readonly<Record<string, UiText>> = {
    "abstract": message("builtinProperty.member.abstract"),
    "accessed": message("builtinProperty.member.accessed"),
    "action": message("builtinProperty.member.action"),
    "address": message("builtinProperty.member.address"),
    "affiliations": message("builtinProperty.member.affiliations"),
    "agent": message("builtinProperty.member.agent"),
    "alternative_titles": message("builtinProperty.member.alternativeTitles"),
    "at": message("builtinProperty.member.at"),
    "before": message("builtinProperty.member.before"),
    "byMonth": message("builtinProperty.member.byMonth"),
    "byMonthDay": message("builtinProperty.member.byMonthDay"),
    "byWeekday": message("builtinProperty.member.byWeekday"),
    "calendar": message("builtinProperty.member.calendar"),
    "calendar_version": message("builtinProperty.member.calendarVersion"),
    "channel": message("builtinProperty.member.channel"),
    "citation_key": message("builtinProperty.member.citationKey"),
    "clock": message("builtinProperty.member.clock"),
    "confidence": message("builtinProperty.member.confidence"),
    "container": message("builtinProperty.member.container"),
    "context": message("builtinProperty.member.context"),
    "count": message("builtinProperty.member.count"),
    "credited_as": message("builtinProperty.member.creditedAs"),
    "custom": message("builtinProperty.member.custom"),
    "customKind": message("builtinProperty.member.customKind"),
    "customLabel": message("builtinProperty.member.customLabel"),
    "day": message("builtinProperty.member.day"),
    "department": message("builtinProperty.member.department"),
    "description": message("builtinProperty.member.description"),
    "during": message("builtinProperty.member.during"),
    "employer": message("builtinProperty.member.employer"),
    "end": message("builtinProperty.member.end"),
    "end_exclusive": message("builtinProperty.member.endExclusive"),
    "entries": message("builtinProperty.member.entries"),
    "event_time": message("builtinProperty.member.eventTime"),
    "exDates": message("builtinProperty.member.exDates"),
    "exceptions": message("builtinProperty.member.exceptions"),
    "family": message("builtinProperty.member.family"),
    "fold": message("builtinProperty.member.fold"),
    "fraction": message("builtinProperty.member.fraction"),
    "frequency": message("builtinProperty.member.frequency"),
    "gap": message("builtinProperty.member.gap"),
    "given": message("builtinProperty.member.given"),
    "id": message("builtinProperty.member.id"),
    "identifier": message("builtinProperty.member.identifier"),
    "interval": message("builtinProperty.member.interval"),
    "issue": message("builtinProperty.member.issue"),
    "issued": message("builtinProperty.member.issued"),
    "key": message("builtinProperty.member.key"),
    "keywords": message("builtinProperty.member.keywords"),
    "kind": message("builtinProperty.member.kind"),
    "label": message("builtinProperty.member.label"),
    "language": message("builtinProperty.member.language"),
    "leap_month": message("builtinProperty.member.leapMonth"),
    "level": message("builtinProperty.member.level"),
    "locator": message("builtinProperty.member.locator"),
    "minutes_before": message("builtinProperty.member.minutesBefore"),
    "month": message("builtinProperty.member.month"),
    "note": message("builtinProperty.member.note"),
    "number": message("builtinProperty.member.number"),
    "observed_at": message("builtinProperty.member.observedAt"),
    "onset": message("builtinProperty.member.onset"),
    "ordinal": message("builtinProperty.member.ordinal"),
    "pages": message("builtinProperty.member.pages"),
    "position": message("builtinProperty.member.position"),
    "precision": message("builtinProperty.member.precision"),
    "preferred": message("builtinProperty.member.preferred"),
    "preset": message("builtinProperty.member.preset"),
    "publication_history": message("builtinProperty.member.publicationHistory"),
    "quantity": message("builtinProperty.member.quantity"),
    "rDates": message("builtinProperty.member.rDates"),
    "rank": message("builtinProperty.member.rank"),
    "ref": message("builtinProperty.member.ref"),
    "reference": message("builtinProperty.member.reference"),
    "relationship": message("builtinProperty.member.relationship"),
    "resource": message("builtinProperty.member.resource"),
    "resources": message("builtinProperty.member.resources"),
    "revision": message("builtinProperty.member.revision"),
    "role": message("builtinProperty.member.role"),
    "roles": message("builtinProperty.member.roles"),
    "rule": message("builtinProperty.member.rule"),
    "schema": message("builtinProperty.member.schema"),
    "scheme": message("builtinProperty.member.scheme"),
    "script": message("builtinProperty.member.script"),
    "selected": message("builtinProperty.member.selected"),
    "series_key": message("builtinProperty.member.seriesKey"),
    "service": message("builtinProperty.member.service"),
    "serviceKey": message("builtinProperty.member.serviceKey"),
    "sources": message("builtinProperty.member.sources"),
    "state": message("builtinProperty.member.state"),
    "status": message("builtinProperty.member.status"),
    "system": message("builtinProperty.member.system"),
    "target": message("builtinProperty.member.target"),
    "text": message("builtinProperty.member.text"),
    "time": message("builtinProperty.member.time"),
    "title": message("builtinProperty.member.title"),
    "unit": message("builtinProperty.member.unit"),
    "until": message("builtinProperty.member.until"),
    "url": message("builtinProperty.member.url"),
    "usage": message("builtinProperty.member.usage"),
    "value": message("builtinProperty.member.value"),
    "venue": message("builtinProperty.member.venue"),
    "version": message("builtinProperty.member.version"),
    "version_of": message("builtinProperty.member.versionOf"),
    "volume": message("builtinProperty.member.volume"),
    "weekStart": message("builtinProperty.member.weekStart"),
    "work": message("builtinProperty.member.work"),
    "year": message("builtinProperty.member.year"),
    "zone": message("builtinProperty.member.zone"),
    "zone_version": message("builtinProperty.member.zoneVersion"),
    "contributors": message("builtinProperty.member.contributors"),
    "identifiers": message("builtinProperty.member.identifiers"),
    "range": message("builtinProperty.member.range"),
    "start": message("builtinProperty.member.start")
};

const choiceMessages: Readonly<Record<string, UiText>> = {
    "personal": message("builtinProperty.choice.personal"),
    "work": message("builtinProperty.choice.work"),
    "other": message("builtinProperty.choice.other"),
    "ordinary": message("builtinProperty.choice.ordinary"),
    "legal": message("builtinProperty.choice.legal"),
    "former": message("builtinProperty.choice.former"),
    "alias": message("builtinProperty.choice.alias"),
    "transliteration": message("builtinProperty.choice.transliteration"),
    "wechat": message("builtinProperty.choice.wechat"),
    "qq": message("builtinProperty.choice.qq"),
    "x": message("builtinProperty.choice.x"),
    "facebook": message("builtinProperty.choice.facebook"),
    "custom": message("builtinProperty.choice.custom"),
    "birth": message("builtinProperty.choice.birth"),
    "death": message("builtinProperty.choice.death"),
    "employment-start": message("builtinProperty.choice.employmentStart"),
    "graduation": message("builtinProperty.choice.graduation"),
    "marriage": message("builtinProperty.choice.marriage"),
    "height": message("builtinProperty.choice.height"),
    "weight": message("builtinProperty.choice.weight"),
    "draft": message("builtinProperty.choice.draft"),
    "accepted": message("builtinProperty.choice.accepted"),
    "published": message("builtinProperty.choice.published"),
    "article": message("builtinProperty.choice.article"),
    "book": message("builtinProperty.choice.book"),
    "report": message("builtinProperty.choice.report"),
    "standard": message("builtinProperty.choice.standard"),
    "dataset": message("builtinProperty.choice.dataset"),
    "chapter": message("builtinProperty.choice.chapter"),
    "thesis": message("builtinProperty.choice.thesis"),
    "web": message("builtinProperty.choice.web"),
    "day": message("builtinProperty.choice.day"),
    "week": message("builtinProperty.choice.week"),
    "month": message("builtinProperty.choice.month"),
    "quarter": message("builtinProperty.choice.quarter"),
    "year": message("builtinProperty.choice.year"),
    "MO": message("builtinProperty.choice.MO"),
    "TU": message("builtinProperty.choice.TU"),
    "WE": message("builtinProperty.choice.WE"),
    "TH": message("builtinProperty.choice.TH"),
    "FR": message("builtinProperty.choice.FR"),
    "SA": message("builtinProperty.choice.SA"),
    "SU": message("builtinProperty.choice.SU"),
    "cancel": message("builtinProperty.choice.cancel"),
    "replace": message("builtinProperty.choice.replace"),
    "tentative": message("builtinProperty.choice.tentative"),
    "confirmed": message("builtinProperty.choice.confirmed"),
    "cancelled": message("builtinProperty.choice.cancelled"),
    "daily": message("builtinProperty.choice.daily"),
    "weekly": message("builtinProperty.choice.weekly"),
    "monthly": message("builtinProperty.choice.monthly"),
    "yearly": message("builtinProperty.choice.yearly"),
    "earlier": message("builtinProperty.choice.earlier"),
    "later": message("builtinProperty.choice.later"),
    "skip": message("builtinProperty.choice.skip"),
    "extended-relative": message("builtinProperty.choice.extendedRelative"),
    "acquaintance": message("builtinProperty.choice.acquaintance"),
    "classmate": message("builtinProperty.choice.classmate"),
    "friend": message("builtinProperty.choice.friend"),
    "colleague": message("builtinProperty.choice.colleague"),
    "absolute": message("builtinProperty.choice.absolute"),
    "due_before": message("builtinProperty.choice.dueBefore"),
    "due_day": message("builtinProperty.choice.dueDay"),
    "open": message("builtinProperty.choice.open"),
    "done": message("builtinProperty.choice.done"),
    "low": message("builtinProperty.choice.low"),
    "normal": message("builtinProperty.choice.normal"),
    "high": message("builtinProperty.choice.high"),
    "urgent": message("builtinProperty.choice.urgent")
};

const builtin = bundled as unknown as DataSchema;
/** Only UI-relevant shape semantics participate. Definition names and object-key order
 * may differ; a custom FieldId, renamed key or changed shape never borrows a caption. */
function shapeMeaning(shape: Shape, schema: DataSchema, seen = new Set<string>()): unknown {
    if (shape.kind === "use") {
        if (seen.has(shape.name) || !schema.definitions[shape.name]) throw new Error("Unresolved presentation shape");
        return shapeMeaning(schema.definitions[shape.name], schema, new Set([...seen, shape.name]));
    }
    if (shape.kind === "record") return [shape.kind, shape.unknown, Object.keys(shape.members).sort().map(key => {
        const member = shape.members[key];
        return [key, member.required, member.nullable, shapeMeaning(member.shape, schema, seen)];
    })];
    if (shape.kind === "list") return [shape.kind, shape.min, shape.max, shapeMeaning(shape.items, schema, seen)];
    if (shape.kind === "union") {
        const d = shape.discriminator;
        return [shape.kind, d && [d.kind, d.keys ?? null, d.key ?? null, d.present ?? null, d.absent ?? null], shape.variants.map(value => shapeMeaning(value, schema, seen))];
    }
    if (shape.kind === "enum") return [shape.kind, shape.open, shape.values];
    return [shape.kind, shape.semantic];
}
const builtinFields = new Map(builtin.fields.map(field => [field.id, {
    key: field.key, shape: JSON.stringify(shapeMeaning(field.shape, builtin)),
}]));
export function isBuiltinField(schema: DataSchema, field: Pick<SchemaField, "id" | "key" | "shape">): boolean {
    const expected = builtinFields.get(field.id);
    if (!expected || field.key !== expected.key) return false;
    try { return JSON.stringify(shapeMeaning(field.shape, schema)) === expected.shape; }
    catch { return false; }
}
export function schemaFieldLabel(schema: DataSchema, field: Pick<SchemaField, "id" | "key" | "shape">): UiText {
    return isBuiltinField(schema, field) && fieldMessages[field.key] ? fieldMessages[field.key] : field.key;
}
function builtinIdentity(schema: DataSchema, identity: NativeControlIdentity): boolean {
    const field = identity.fieldId && schema.fields.find(field => field.id === identity.fieldId);
    return !!field && isBuiltinField(schema, field);
}
/** One root form owns one qualification. Recursive editors share this context;
 * a replacement schema/form gets a new context instead of a cross-schema cache. */
export function schemaPropertyPresentation(schema: DataSchema, identity: NativeControlIdentity) {
    const builtin = builtinIdentity(schema, identity);
    return {
        memberLabel: (key: string): UiText => builtin && memberMessages[key] ? memberMessages[key] : key,
        enumLabel: (shape: Extract<Shape, { kind: "enum" }>, value: string): UiText => builtin && shape.values.includes(value) && choiceMessages[value] ? choiceMessages[value] : value,
    };
}
export function schemaMemberLabel(schema: DataSchema, identity: NativeControlIdentity, key: string): UiText {
    return schemaPropertyPresentation(schema, identity).memberLabel(key);
}
export function schemaEnumLabel(schema: DataSchema, identity: NativeControlIdentity, shape: Extract<Shape, { kind: "enum" }>, value: string): UiText {
    return schemaPropertyPresentation(schema, identity).enumLabel(shape, value);
}
