import { resolveShape, unionVariant, type DataSchema, type NativeValue, type Shape } from "./native-value-form";
import { message, UiError } from "./ui-locale";

export const queryTextModes = {
    all: message("queryText.all"),
    any: message("queryText.any"),
    phrase: message("queryText.phrase"),
    exclude: message("queryText.exclude"),
};
export type QueryTextMode = keyof typeof queryTextModes;
export function queryTextPredicate(expr: string, query: string, mode: QueryTextMode): string {
    return `textMatch(${expr}, ${JSON.stringify(query)}, ${JSON.stringify(mode)})`;
}

const labels = {
    "==": message("queryOperator.equals"), "!=": message("queryOperator.notEquals"),
    "<": message("queryOperator.lessThan"), "<=": message("queryOperator.atMost"),
    ">": message("queryOperator.greaterThan"), ">=": message("queryOperator.atLeast"),
    contains: message("queryOperator.containsText"), startsWith: message("queryOperator.startsWith"), endsWith: message("queryOperator.endsWith"),
    all: queryTextModes.all, any: queryTextModes.any, phrase: queryTextModes.phrase, exclude: queryTextModes.exclude,
    member: message("queryOperator.hasItem"), empty: message("queryOperator.empty"),
    present: message("queryOperator.present"), absent: message("queryValue.absent"), null: message("queryValue.explicitNull"),
    invalid: message("queryValue.invalid"), unavailable: message("queryValue.unavailable"),
};
type Operator = keyof typeof labels;
const textSemantics = new Set(["text", "nonempty_text", "phone", "email", "identifier", "doi", "url", "language_tag", "citation_key", "config_revision"]);

function valueOperators(shape: Shape, schema: DataSchema): Operator[] {
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "union") {
        const variants = resolved.variants.map(variant => valueOperators(variant, schema));
        return variants[0]?.filter(op => op !== "empty" && op !== "member" && variants.every(ops => ops.includes(op))) ?? [];
    }
    if (resolved.kind === "list") {
        const item = resolveShape(resolved.items, schema);
        return item.kind === "scalar" || item.kind === "enum" ? ["member", "empty"] : ["empty"];
    }
    if (resolved.kind === "record") return ["empty"];
    if (resolved.kind !== "enum" && resolved.kind !== "scalar") return [];
    if (resolved.kind === "scalar" && ["schema_ref", "none"].includes(resolved.semantic)) return [];
    const operators: Operator[] = ["==", "!="];
    if (resolved.kind === "enum" || resolved.semantic !== "bool") operators.push("<", "<=", ">", ">=");
    if (resolved.kind === "enum" || textSemantics.has(resolved.semantic))
        operators.push("contains", "startsWith", "endsWith", "empty", "all", "any", "phrase", "exclude");
    return operators;
}

/** Only direct schema fields have observable projection states. */
export function filterOperators(shape: Shape, schema: DataSchema, directField: boolean) {
    const operators = valueOperators(shape, schema);
    if (directField) operators.push("present", "absent", "null", "invalid", "unavailable");
    return operators.map(value => ({ value, label: labels[value] }));
}

export function filterOperandShape(op: string, shape: Shape, schema: DataSchema): Shape | undefined {
    if (["present", "absent", "null", "invalid", "unavailable", "empty"].includes(op)) return;
    const resolved = resolveShape(shape, schema);
    if (op === "member" && resolved.kind === "list") return resolveShape(resolved.items, schema);
    if (["contains", "startsWith", "endsWith"].includes(op) || Object.hasOwn(queryTextModes, op))
        return { kind: "scalar", semantic: "text" };
    return resolved;
}

export function queryPredicate(expr: string, shape: Shape, schema: DataSchema, directField: boolean, op: string, value?: NativeValue): string {
    if (!filterOperators(shape, schema, directField).some(option => option.value === op))
        throw new UiError(message("queryValidation.unsupportedOperator"));
    switch (op) {
        case "present": return `!isAbsent(${expr})`;
        case "absent": return `isAbsent(${expr})`;
        case "null": return `isNull(${expr})`;
        case "invalid": return `isInvalid(${expr})`;
        case "unavailable": return `isUnavailable(${expr})`;
        case "empty": return `size(${expr}) == 0`;
    }
    const operand = filterOperandShape(op, shape, schema);
    if (!operand || !value) throw new UiError(message("queryValidation.scalarRepresentationMismatch"));
    if (["contains", "startsWith", "endsWith"].includes(op)) {
        if (value.kind !== "text") throw new UiError(message("queryValidation.containsTextValueRequired"));
        return `${expr}.${op}(${JSON.stringify(value.value)})`;
    }
    if (Object.hasOwn(queryTextModes, op)) {
        if (value.kind !== "text") throw new UiError(message("queryValidation.containsTextValueRequired"));
        return queryTextPredicate(expr, value.value, op as QueryTextMode);
    }
    const literal = shapedQueryLiteral(operand, schema, value);
    return op === "member" ? `${literal} in ${expr}` : `${expr} ${op} ${literal}`;
}

export function scalarQueryLiteral(value: NativeValue): string {
    if (value.kind === "none")
        return "null";
    if (value.kind === "bool")
        return String(value.value);
    if (value.kind === "int")
        return value.value;
    if (value.kind === "float")
        return /[.eE]/.test(value.value) ? value.value : `${value.value}.0`;
    if (value.kind === "decimal")
        return `decimal(${JSON.stringify(value.value)})`;
    if (value.kind === "date")
        return `date(${JSON.stringify(value.value)})`;
    if (value.kind === "time")
        return `time(${JSON.stringify(value.value)})`;
    if (value.kind === "localDatetime")
        return `localDatetime(${JSON.stringify(value.value)})`;
    if (value.kind === "duration")
        return `duration(${JSON.stringify(value.value)})`;
    if (value.kind === "text")
        return JSON.stringify(value.value);
    throw new UiError(message("queryValidation.nestedLiteralUnsupported"));
}
export function shapedQueryLiteral(shape: Shape, schema: DataSchema, value: NativeValue): string {
    const resolved = resolveShape(shape, schema);
    if (resolved.kind === "union") {
        const branch = unionVariant(resolved, value, schema);
        if (branch < 0)
            throw new UiError(message("queryValidation.ambiguousValueBranch"));
        return shapedQueryLiteral(resolved.variants[branch], schema, value);
    }
    if (resolved.kind === "scalar") {
        const constructor = ({ node_ref: "nodeRef", tag_ref: "tagRef", resource_ref: "resourceRef", instant: "instant", exact_decimal: "exactDecimal", exact_seconds: "exactSeconds" } as Record<string, string>)[resolved.semantic];
        if (constructor) {
            if (value.kind !== "text")
                throw new UiError(message("queryValidation.scalarRepresentationMismatch"));
            return `${constructor}(${JSON.stringify(value.value)})`;
        }
    }
    return scalarQueryLiteral(value);
}
