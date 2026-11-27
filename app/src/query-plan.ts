import type { Shape } from "./native-value-form";
import { decodeNativeValue, type NativeValue } from "./native-values";
import { displayMessage, message, uiDisplayParameter, UiError } from "./ui-locale";

export interface QueryPlan {
    scope: {
        directory?: string;
        descendants: boolean;
        lifecycle: "live" | "trash";
    };
    types: string[];
    where?: string;
    select: {
        name: string;
        expr: string;
    }[];
    order: {
        expr: string;
        descending: boolean;
        nulls: "first" | "last";
    }[];
    take?: number;
    group?: {
        keys: {
            name: string;
            expr: string;
        }[];
        aggregates: {
            name: string;
            op: "count" | "sum" | "min" | "max";
            expr?: string;
        }[];
    };
    unnest?: {
        array: string;
        binding: string;
    };
    parameters?: {
        name: string;
        shape: Shape;
        nullable?: boolean;
        default?: NativeValue;
    }[];
    advanced?: {
        unionAll?: QueryPlan[];
        distinct?: {
            name: string;
            expr: string;
        }[];
        related?: {
            relation: string;
            from: QueryPlan;
            direction?: "forward" | "inverse";
        };
        call?: {
            owner: string;
            selection: {
                mode: "current";
                key: string;
            } | {
                mode: "pinned";
                key: string;
                revision: string;
            };
            arguments?: Record<string, string>;
        };
    };
}
/** Reject duplicate JSON members before parsing: a plan editor must not silently choose the last value. */
export function parseQueryJson(source: string): unknown {
    const parsed: unknown = JSON.parse(source);
    let position = 0;
    const whitespace = (): void => {
        while (/\s/.test(source[position] ?? "") && position < source.length)
            position++;
    };
    const string = (): string => {
        const start = position++;
        while (position < source.length) {
            const char = source[position++];
            if (char === "\\")
                position++;
            else if (char === '"')
                return JSON.parse(source.slice(start, position)) as string;
        }
        throw new UiError(message("queryValidation.incompleteJsonString"));
    };
    const value = (depth: number): void => {
        if (depth > 100)
            throw new UiError(message("queryValidation.jsonNestingExceeded"));
        whitespace();
        const char = source[position];
        if (char === '"') {
            string();
            return;
        }
        if (char === "{") {
            position++;
            whitespace();
            const keys = new Set<string>();
            if (source[position] === "}") {
                position++;
                return;
            }
            for (;;) {
                whitespace();
                const key = string();
                if (keys.has(key))
                    throw new UiError(displayMessage("queryValidation.duplicateJsonMember", { p0: uiDisplayParameter(key) }));
                keys.add(key);
                whitespace();
                position++;
                value(depth + 1);
                whitespace();
                if (source[position++] === "}")
                    return;
            }
        }
        if (char === "[") {
            position++;
            whitespace();
            if (source[position] === "]") {
                position++;
                return;
            }
            for (;;) {
                value(depth + 1);
                whitespace();
                if (source[position++] === "]")
                    return;
            }
        }
        while (position < source.length && !/[\s,}\]]/.test(source[position]))
            position++;
    };
    value(0);
    return parsed;
}
/** Structural checks only; the shared compiler owns CEL, schema and producer validation. */
export function parseQueryPlan(source: string): QueryPlan {
    const parsed = parseQueryJson(source);
    let nodes = 0;
    const visit = (value: unknown, depth: number): QueryPlan => {
        if (++nodes > 1024 || depth > 64)
            throw new UiError(message("queryValidation.planBudgetExceeded"));
        if (!value || typeof value !== "object" || Array.isArray(value))
            throw new UiError(message("queryValidation.planObjectRequired"));
        const plan = value as Record<string, unknown>;
        const allowed = ["scope", "types", "where", "select", "order", "take", "group", "unnest", "parameters", "advanced"];
        const unknown = Object.keys(plan).find(key => !allowed.includes(key));
        if (unknown)
            throw new UiError(displayMessage("queryValidation.unknownPlanMember", { p0: uiDisplayParameter(unknown) }));
        const scope = plan.scope ?? { descendants: true, lifecycle: "live" };
        if (!scope || typeof scope !== "object" || Array.isArray(scope))
            throw new UiError(message("queryValidation.scopeObjectRequired"));
        const actualScope = scope as Record<string, unknown>;
        if (Object.keys(actualScope).some(key => !["directory", "descendants", "lifecycle"].includes(key)) || actualScope.directory != null && typeof actualScope.directory !== "string" || actualScope.descendants != null && typeof actualScope.descendants !== "boolean" || actualScope.lifecycle != null && !["live", "trash"].includes(String(actualScope.lifecycle)))
            throw new UiError(message("queryValidation.invalidScope"));
        if ("parameters" in plan) {
            if (!Array.isArray(plan.parameters))
                throw new UiError(message("queryValidation.invalidParameterDeclaration"));
            const names = new Set<string>();
            for (const declaration of plan.parameters) {
                if (!declaration || typeof declaration !== "object" || Array.isArray(declaration)
                    || typeof declaration.name !== "string" || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(declaration.name)
                    || ("default" in declaration && !decodeNativeValue(declaration.default, 64))
                    || names.has(declaration.name) || !declaration.shape || typeof declaration.shape.kind !== "string"
                    || ("nullable" in declaration && typeof declaration.nullable !== "boolean")
                    || Object.keys(declaration).some(key => !["name", "shape", "nullable", "default"].includes(key)))
                    throw new UiError(message("queryValidation.invalidParameterDeclaration"));
                names.add(declaration.name);
            }
        }
        if (plan.types != null && (!Array.isArray(plan.types) || plan.types.some(item => typeof item !== "string")))
            throw new UiError(message("queryValidation.typeArrayRequired"));
        for (const key of ["select", "order"])
            if (plan[key] != null && (!Array.isArray(plan[key]) || (plan[key] as unknown[]).some(item => !item || typeof item !== "object" || typeof (item as Record<string, unknown>).expr !== "string")))
                throw new UiError(displayMessage("queryValidation.expressionArrayRequired", { p0: uiDisplayParameter(key) }));
        if (plan.where != null && typeof plan.where !== "string" || plan.take != null && (!Number.isSafeInteger(plan.take) || Number(plan.take) < 1))
            throw new UiError(message("queryValidation.invalidWhereOrTake"));
        const advanced = plan.advanced as QueryPlan["advanced"];
        if (advanced != null) {
            if (typeof advanced !== "object" || Array.isArray(advanced))
                throw new UiError(message("queryValidation.advancedObjectRequired"));
            const unknown = Object.keys(advanced).find(key => !["unionAll", "distinct", "related", "call"].includes(key));
            if (unknown)
                throw new UiError(displayMessage("queryValidation.unknownPlanMember", { p0: uiDisplayParameter(unknown) }));
            if ("unionAll" in advanced) {
                if (!Array.isArray(advanced.unionAll) || !advanced.unionAll.length)
                    throw new UiError(message("queryValidation.unionSourcesRequired"));
                advanced.unionAll = advanced.unionAll.map(branch => visit(branch, depth + 1));
            }
            if (advanced.related)
                advanced.related.from = visit(advanced.related.from, depth + 1);
        }
        return { ...plan, scope: { ...actualScope, descendants: actualScope.descendants ?? true, lifecycle: actualScope.lifecycle ?? "live" }, types: plan.types ?? [], select: plan.select ?? [], order: plan.order ?? [] } as QueryPlan;
    };
    return visit(parsed, 0);
}
export function parseQueryParameters(source: string): Record<string, NativeValue> {
    const parsed = parseQueryJson(source);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
        throw new UiError(message("queryValidation.parameterObjectRequired"));
    return Object.fromEntries(Object.entries(parsed).map(([name, value]) => {
        const native = decodeNativeValue(value, 64);
        if (!native) throw new UiError(message("queryValidation.invalidNativeParameter"));
        return [name, native];
    }));
}
