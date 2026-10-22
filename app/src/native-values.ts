/** Author values are independent of product display messages and interface language. */
export type NativeValue = {kind:"none"} | {kind:"text"|"int"|"float"|"decimal"|"date"|"time"|"localDatetime"|"duration";value:string} | {kind:"bool";value:boolean} | {kind:"array";value:NativeValue[]} | {kind:"record";value:{key:string;value:NativeValue}[]};

export function decodeNativeValue(value: unknown): NativeValue | undefined {
    const ancestors = new Set<object>();
    const visit = (value: unknown, depth: number): NativeValue | undefined => {
        if (!value || typeof value !== "object" || Array.isArray(value) || depth > 256 || ancestors.has(value)) return;
        const item = value as Record<string,unknown>;
        if (item.kind === "none") return Object.keys(item).length === 1 ? {kind:"none"} : undefined;
        if (Object.keys(item).sort().join(",") !== "kind,value") return;
        if (["text","int","float","decimal","date","time","localDatetime","duration"].includes(String(item.kind))) return typeof item.value === "string" ? {kind:item.kind as "text",value:item.value} : undefined;
        if (item.kind === "bool") return typeof item.value === "boolean" ? {kind:"bool",value:item.value} : undefined;
        if (!Array.isArray(item.value)) return;
        ancestors.add(value);
        try {
            if (item.kind === "array") {
                const children: NativeValue[] = [];
                for (const child of item.value) { const decoded = visit(child,depth+1); if (!decoded) return; children.push(decoded); }
                return {kind:"array",value:children};
            }
            if (item.kind === "record") {
                const members: {key:string;value:NativeValue}[] = [];
                for (const child of item.value) {
                    if (!child || typeof child !== "object" || Array.isArray(child) || Object.keys(child).sort().join(",") !== "key,value" || typeof child.key !== "string") return;
                    const decoded = visit(child.value,depth+1); if (!decoded) return; members.push({key:child.key,value:decoded});
                }
                return {kind:"record",value:members};
            }
        } finally { ancestors.delete(value); }
    };
    return visit(value,0);
}
