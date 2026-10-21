/** Protocol failures stay machine facts; the caller owns their product explanation. */
export class StrictJsonError extends Error {
    constructor(readonly reason: "duplicateMember" | "nestingBudget") { super(reason); this.name = "StrictJsonError"; }
}

/** JSON.parse supplies grammar/number semantics. This pass rejects information it would discard. */
export function parseStrictJson(source: string): unknown {
    const parsed: unknown = JSON.parse(source);
    let position = 0;
    const whitespace = () => { while (position < source.length && /\s/.test(source[position])) position++; };
    const string = (): string => {
        const start = position++;
        while (position < source.length) {
            const character = source[position++];
            if (character === "\\") position++;
            else if (character === '"') return JSON.parse(source.slice(start, position)) as string;
        }
        throw new SyntaxError("Unterminated JSON string");
    };
    const value = (depth: number): void => {
        if (depth > 256) throw new StrictJsonError("nestingBudget");
        whitespace();
        if (source[position] === '"') { string(); return; }
        if (source[position] === "{") {
            position++; whitespace(); const names = new Set<string>();
            if (source[position] === "}") { position++; return; }
            for (;;) {
                whitespace(); const name = string();
                if (names.has(name)) throw new StrictJsonError("duplicateMember");
                names.add(name); whitespace(); position++; value(depth + 1); whitespace();
                if (source[position++] === "}") return;
            }
        }
        if (source[position] === "[") {
            position++; whitespace();
            if (source[position] === "]") { position++; return; }
            for (;;) { value(depth + 1); whitespace(); if (source[position++] === "]") return; }
        }
        while (position < source.length && !/[\s,}\]]/.test(source[position])) position++;
    };
    value(0);
    return parsed;
}

/** Both local and hosted transports use the raw response before duplicate members can disappear. */
export async function readStrictJsonResponse(response: Pick<Response, "text">): Promise<unknown> {
    return parseStrictJson(await response.text());
}
