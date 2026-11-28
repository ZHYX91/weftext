import { action, ui } from "./native-value-form";
import type { QueryPlan } from "./query-plan";
import { message, setUiAttr, type UiText } from "./ui-locale";

type Direction = "forward" | "inverse";
type Related = NonNullable<NonNullable<QueryPlan["advanced"]>["related"]>;

function hops(plan: QueryPlan): Related[] {
    const traversals: Related[] = [];
    let current = plan;
    while (current.advanced?.related && traversals.length <= 2) {
        traversals.push(current.advanced.related);
        current = current.advanced.related.from;
    }
    return traversals.reverse();
}

export function followQueryRelation(plan: QueryPlan, relation: string, direction: Direction): QueryPlan {
    const source = structuredClone(plan);
    return {
        scope: structuredClone(source.scope),
        types: [],
        parameters: structuredClone(source.parameters),
        select: structuredClone(source.select),
        order: [],
        advanced: { related: { relation, direction, from: source } },
    };
}

export function queryRelationControls(
    plan: QueryPlan, relations: readonly string[], replace: (plan: QueryPlan) => void,
): HTMLFieldSetElement {
    const host = ui("fieldset", "query-relation-controls");
    const related = hops(plan);
    host.append(ui("legend", "", message("queryRelations.title")));
    if (related.length > 2) {
        host.append(ui("p", "data-help", message("queryRelations.useSource")));
        return host;
    }
    const choice = (label: UiText, options: [string, UiText][], selected: string) => {
        const result = ui("select", "data-input");
        setUiAttr(result, "aria-label", label);
        for (const [id, text] of options) {
            const option = ui("option", "", text);
            option.value = id;
            result.append(option);
        }
        result.value = selected;
        return result;
    };
    const relationOptions = relations.map((id): [string, UiText] => [id, id]);
    const directions: [Direction, UiText][] = [
        ["forward", message("queryRelations.forward")],
        ["inverse", message("queryRelations.inverse")],
    ];
    for (const [index, hop] of related.entries()) {
        const row = ui("fieldset");
        row.append(ui("legend", "", index === 0 ? message("queryRelations.firstHop") : message("queryRelations.secondHop")));
        const options = relationOptions.some(([id]) => id === hop.relation)
            ? relationOptions : [...relationOptions, [hop.relation, hop.relation] as [string, UiText]];
        const relation = choice(message("queryRelations.relation"), options, hop.relation);
        const direction = choice(message("queryRelations.direction"), directions, hop.direction ?? "forward");
        row.append(relation, direction, action(message("queryRelations.updateHop"), () => {
            const proposal = structuredClone(plan);
            const selected = hops(proposal)[index];
            selected.relation = relation.value;
            selected.direction = direction.value as Direction;
            replace(proposal);
        }));
        host.append(row);
    }
    const last = plan.advanced?.related;
    if (last)
        host.append(action(message("queryRelations.backToSource"), () => replace(structuredClone(last.from))));
    if (related.length < 2 && relations.length) {
        const relation = choice(message("queryRelations.nextRelation"), relationOptions, relations[0]);
        const direction = choice(message("queryRelations.direction"), directions, "forward");
        host.append(relation, direction, action(message("queryRelations.addHop"), () =>
            replace(followQueryRelation(plan, relation.value, direction.value as Direction))));
    }
    return host;
}
