import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { TagTree, type TreeTag, type TagMatchMode } from "./tag-tree";
export interface GraphNode {
    key: string;
    reference: string;
    title: string;
    directory: string;
    tags: string[];
    tagsKnown?: boolean;
}
export interface GraphEdge {
    key: string;
    source: string;
    target: string;
    relation: string;
    direction: "directed" | "symmetric";
    derived: boolean;
    authorSource: string;
    fieldId?: string | null;
    itemPath: string[];
    range?: {
        utf16From: number;
        utf16To: number;
    } | null;
    sourceHash: string;
}
export interface GraphSnapshot {
    generation: string;
    root?: string;
    tagTree?: TreeTag[] | null;
    scope: "authorizedNativeSourcesAndTypedReferences";
    coverage: "complete" | "partial";
    truncated: boolean;
    nodes: GraphNode[];
    edges: GraphEdge[];
}
export interface GraphFilters {
    scope: "local" | "global";
    root: string;
    direction: "in" | "out" | "both";
    depth: number;
    relation: string;
    directory: string;
    tag: string;
    tagMode: TagMatchMode;
    expanded: string[];
}
export interface Point {
    x: number;
    y: number;
}
export interface Camera extends Point {
    scale: number;
}
export const defaultCamera = (): Camera => ({ x: 0, y: 0, scale: 1 });
export const defaultGraphFilters = (root: string): GraphFilters => ({ scope: "local", root, direction: "both", depth: 1, relation: "", directory: "", tag: "", tagMode: "subtree", expanded: [] });
/** All numbers describe only supplied authorized nodes. Search deliberately is
 * absent: locating a result must not silently widen the declared graph scope. */
export function graphVisible(snapshot: GraphSnapshot, filter: GraphFilters): {
    nodes: GraphNode[];
    edges: GraphEdge[];
    tagIncomplete: boolean;
} {
    const tree = filter.tag && snapshot.tagTree ? new TagTree(snapshot.tagTree, true) : null;
    if (filter.tag && (!tree || !tree.byRef.has(filter.tag)))
        throw new UiError(uiMessage("relationshipGraphState.treeOrRootUnavailable"));
    let tagIncomplete = false;
    const nodes = snapshot.nodes.filter(n => {
        if (filter.directory && n.directory !== filter.directory && !n.directory.startsWith(`${filter.directory}/`))
            return false;
        if (!filter.tag)
            return true;
        if (!n.tagsKnown) {
            tagIncomplete = true;
            return false;
        }
        let matches = false;
        for (const reference of n.tags) {
            const tag = tree!.byRef.get(reference);
            if (!tag) {
                tagIncomplete = true;
                continue;
            }
            if (filter.tagMode === "exact" ? reference === filter.tag : tree!.contains(tree!.byRef.get(filter.tag)!.tagId, tag.tagId))
                matches = true;
        }
        return matches;
    });
    const allowed = new Set(nodes.map(n => n.key)), edges = snapshot.edges.filter(e => allowed.has(e.source) && allowed.has(e.target) && (!filter.relation || e.relation === filter.relation));
    if (filter.scope === "global")
        return { nodes, edges, tagIncomplete };
    const adjacent = new Map<string, Set<string>>();
    const add = (from: string, to: string) => {
        let values = adjacent.get(from);
        if (!values) {
            values = new Set();
            adjacent.set(from, values);
        }
        values.add(to);
    };
    for (const edge of edges) {
        if (filter.direction !== "in" || edge.direction === "symmetric")
            add(edge.source, edge.target);
        if (filter.direction !== "out" || edge.direction === "symmetric")
            add(edge.target, edge.source);
    }
    const reached = new Set<string>(), queue: [
        string,
        number
    ][] = [[filter.root, Math.min(8, Math.max(0, filter.depth))]];
    // Expansion is explicit and bounded to one further neighborhood per click.
    queue.push(...filter.expanded.filter(key => allowed.has(key)).map(key => [key, 1] as [
        string,
        number
    ]));
    const budgets = new Map<string, number>();
    for (let i = 0; i < queue.length; i++) {
        const [key, remaining] = queue[i];
        if (!allowed.has(key) || (budgets.get(key) ?? -1) >= remaining)
            continue;
        budgets.set(key, remaining);
        reached.add(key);
        if (remaining > 0)
            for (const next of adjacent.get(key) ?? [])
                queue.push([next, remaining - 1]);
    }
    return { nodes: nodes.filter(n => reached.has(n.key)), edges: edges.filter(e => reached.has(e.source) && reached.has(e.target)), tagIncomplete };
}
export function graphMatches(nodes: GraphNode[], text: string): GraphNode[] { const needle = text.trim().toLocaleLowerCase(); return needle ? nodes.filter(n => `${n.title}\n${n.key}`.toLocaleLowerCase().includes(needle)) : []; }
export function graphLayout(nodes: Pick<GraphNode, "key">[], edges: Pick<GraphEdge, "source" | "target" | "direction">[], mode: "general" | "ranked" | "family", existing: Map<string, Point> = new Map()): {
    positions: Map<string, Point>;
    cycle: boolean;
} {
    const positions = new Map<string, Point>(), ids = new Set(nodes.map(n => n.key));
    const indegree = new Map(nodes.map(n => [n.key, 0])), adjacent = new Map<string, string[]>(), levels = new Map<string, number>();
    for (const e of edges) {
        if (!ids.has(e.source) || !ids.has(e.target) || e.direction === "symmetric")
            continue;
        indegree.set(e.target, indegree.get(e.target)! + 1);
        const list = adjacent.get(e.source) ?? [];
        list.push(e.target);
        adjacent.set(e.source, list);
    }
    const queue = nodes.filter(n => indegree.get(n.key) === 0).map(n => n.key);
    for (let i = 0; i < queue.length; i++)
        for (const target of adjacent.get(queue[i]) ?? []) {
            levels.set(target, Math.max(levels.get(target) ?? 0, (levels.get(queue[i]) ?? 0) + 1));
            indegree.set(target, indegree.get(target)! - 1);
            if (!indegree.get(target))
                queue.push(target);
        }
    const cycle = queue.length < nodes.length, rows = new Map<number, number>();
    nodes.forEach((n, index) => {
        const old = existing.get(n.key);
        if (old) {
            positions.set(n.key, { ...old });
            return;
        }
        if (mode !== "general" && !cycle) {
            const level = levels.get(n.key) ?? 0, row = rows.get(level) ?? 0;
            rows.set(level, row + 1);
            positions.set(n.key, mode === "family" ? { x: row * 180 + 100, y: level * 115 + 65 } : { x: level * 190 + 100, y: row * 85 + 65 });
        }
        else {
            const columns = Math.max(1, Math.ceil(Math.sqrt(nodes.length)));
            positions.set(n.key, { x: index % columns * 165 + 100, y: Math.floor(index / columns) * 100 + 65 });
        }
    });
    return { positions, cycle };
}
export function fitCamera(points: Point[], width: number, height: number): Camera {
    if (!points.length)
        return defaultCamera();
    const xs = points.map(p => p.x), ys = points.map(p => p.y), left = Math.min(...xs) - 95, right = Math.max(...xs) + 95, top = Math.min(...ys) - 35, bottom = Math.max(...ys) + 35, scale = Math.min(1.6, Math.max(.1, Math.min(width / (right - left), height / (bottom - top))));
    return { scale, x: (width - (right + left) * scale) / 2, y: (height - (bottom + top) * scale) / 2 };
}
export function zoomCamera(camera: Camera, factor: number, anchor: Point): Camera { const scale = Math.max(.1, Math.min(4, camera.scale * factor)), ratio = scale / camera.scale; return { scale, x: anchor.x - (anchor.x - camera.x) * ratio, y: anchor.y - (anchor.y - camera.y) * ratio }; }
