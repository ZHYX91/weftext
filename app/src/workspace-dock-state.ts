/** Layout preferences only. No authored content, resolved results, credentials or writers. */
export type DockSide = "left" | "right";

export interface DockGroup {
    id: string;
    views: string[];
    active: string;
    weight: number;
    collapsed: boolean;
}
export interface DockHiddenPlacement {
    side: DockSide;
    groupId: string;
    groupIndex: number;
    index: number;
    weight?: number;
}
export interface DockState {
    version: 1;
    left: DockGroup[];
    right: DockGroup[];
    hidden: string[];
    hiddenPlacement: Record<string, DockHiddenPlacement>;
    maximized: string | null;
    leftHidden: boolean;
    rightHidden: boolean;
}
export interface DockViewDefinition {
    id: string;
    side: DockSide;
    group?: string;
    hidden?: boolean;
}
export interface DockMoveTarget {
    side: DockSide;
    groupId?: string;
    position?: "before" | "after" | "center";
}
const sides: readonly DockSide[] = ["left", "right"];
const maxGroups = 30;
const validId = (value: unknown): value is string => typeof value === "string" && value.length > 0 && value.length <= 100 && !/[\u0000-\u001f\u007f]/u.test(value) && !["__proto__", "prototype", "constructor"].includes(value);
const object = (value: unknown): Record<string, unknown> | null => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
const weight = (value: unknown): number => typeof value === "number" && Number.isFinite(value) ? Math.min(10, Math.max(.1, value)) : 1;
const index = (value: unknown, max: number): number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? Math.min(max, value) : 0;
const definitions = (views: readonly DockViewDefinition[]): DockViewDefinition[] => views.filter((view, at) => validId(view.id) && sides.includes(view.side) && views.findIndex(item => item.id === view.id) === at);
const emptyPlacements = (): Record<string, DockHiddenPlacement> => Object.create(null) as Record<string, DockHiddenPlacement>;
function clone(state: DockState): DockState {
    const hiddenPlacement = emptyPlacements();
    for (const [id, placement] of Object.entries(state.hiddenPlacement)) hiddenPlacement[id] = { ...placement };
    return { ...state, left: state.left.map(group => ({ ...group, views: [...group.views] })), right: state.right.map(group => ({ ...group, views: [...group.views] })), hidden: [...state.hidden], hiddenPlacement };
}
function freshGroupId(state: DockState, preferred: string): string {
    const occupied = new Set([...state.left, ...state.right].map(group => group.id));
    const base = validId(preferred) ? preferred : "dock-group";
    if (!occupied.has(base)) return base;
    for (let suffix = 2; ; suffix++) {
        const candidate = `${base.slice(0, 88)}-${suffix}`;
        if (!occupied.has(candidate)) return candidate;
    }
}
function appendDefault(state: DockState, view: DockViewDefinition): void {
    const requested = validId(view.group) ? view.group : `${view.side}-main`;
    // A group name may be reused on opposite sides; make that side's canonical
    // name deterministic so later registrations join it rather than another group.
    const preferred = state[view.side === "left" ? "right" : "left"].some(group => group.id === requested) ? `${view.side}-${requested}`.slice(0, 100) : requested;
    let group = state[view.side].find(item => item.id === preferred);
    if (!group) {
        if (state[view.side].length >= maxGroups) group = state[view.side].at(-1)!;
        else {
            group = { id: freshGroupId(state, preferred), views: [], active: view.id, weight: 1, collapsed: false };
            state[view.side].push(group);
        }
    }
    group.views.push(view.id);
}
export function defaultDockState(views: readonly DockViewDefinition[]): DockState {
    const state: DockState = { version: 1, left: [], right: [], hidden: [], hiddenPlacement: emptyPlacements(), maximized: null, leftHidden: false, rightHidden: false };
    for (const view of definitions(views)) {
        if (view.hidden) state.hidden.push(view.id);
        else appendDefault(state, view);
    }
    state.rightHidden = state.right.length === 0;
    return state;
}
/** Decode only registered view IDs, bounded geometry and grouping; never retain arbitrary keys. */
export function decodeDockState(raw: unknown, views: readonly DockViewDefinition[]): DockState {
    const item = object(raw), registered = definitions(views);
    if (!item || item.version !== 1) return defaultDockState(registered);
    const state: DockState = { version: 1, left: [], right: [], hidden: [], hiddenPlacement: emptyPlacements(), maximized: null, leftHidden: item.leftHidden === true, rightHidden: item.rightHidden === true };
    const allowed = new Set(registered.map(view => view.id)), used = new Set<string>();
    for (const side of sides) {
        if (!Array.isArray(item[side])) continue;
        for (const value of item[side].slice(0, maxGroups)) {
            const group = object(value);
            if (!group || !validId(group.id) || !Array.isArray(group.views)) continue;
            const members: string[] = [];
            for (const id of group.views.slice(0, Math.max(allowed.size * 2, 100))) {
                if (!validId(id) || !allowed.has(id) || used.has(id)) continue;
                used.add(id); members.push(id);
            }
            if (!members.length) continue;
            state[side].push({ id: freshGroupId(state, group.id), views: members, active: typeof group.active === "string" && members.includes(group.active) ? group.active : members[0], weight: weight(group.weight), collapsed: group.collapsed === true });
        }
    }
    if (Array.isArray(item.hidden)) {
        for (const id of item.hidden.slice(0, Math.max(allowed.size * 2, 100))) {
            if (!validId(id) || !allowed.has(id) || used.has(id)) continue;
            used.add(id); state.hidden.push(id);
        }
    }
    for (const view of registered) {
        if (used.has(view.id)) continue;
        if (view.hidden) state.hidden.push(view.id);
        else appendDefault(state, view);
    }
    const placements = object(item.hiddenPlacement);
    if (placements) for (const id of state.hidden) {
        if (!Object.hasOwn(placements, id)) continue;
        const placement = object(placements[id]);
        if (!placement || !sides.includes(placement.side as DockSide) || !validId(placement.groupId)) continue;
        state.hiddenPlacement[id] = { side: placement.side as DockSide, groupId: placement.groupId, groupIndex: index(placement.groupIndex, maxGroups - 1), index: index(placement.index, allowed.size),
            ...(typeof placement.weight === "number" && Number.isFinite(placement.weight) ? { weight: weight(placement.weight) } : {}) };
    }
    if (typeof item.maximized === "string" && [...state.left, ...state.right].some(group => group.id === item.maximized && !group.collapsed)) state.maximized = item.maximized;
    state.rightHidden = typeof item.rightHidden === "boolean" ? item.rightHidden : state.right.length === 0;
    return state;
}
function removeView(state: DockState, id: string): void {
    for (const side of sides) {
        state[side] = state[side].flatMap(group => {
            const at = group.views.indexOf(id);
            if (at < 0) return [group];
            group.views.splice(at, 1);
            if (!group.views.length) {
                if (state.maximized === group.id) state.maximized = null;
                return [];
            }
            if (group.active === id) group.active = group.views[Math.min(at, group.views.length - 1)];
            return [group];
        });
    }
    state.hidden = state.hidden.filter(view => view !== id);
}
/** Hidden members retain logical slots, so hiding several tabs/groups in succession
 * does not reverse them when they are later revealed in a different order. */
function logicalIndex(visibleIndex: number, hiddenIndices: number[]): number {
    let result = visibleIndex;
    for (const hidden of [...new Set(hiddenIndices)].sort((a, b) => a - b)) if (hidden <= result) result++;
    return result;
}
function missingGroupPlacements(state: DockState, side: DockSide): DockHiddenPlacement[] {
    const seen = new Set<string>();
    return Object.values(state.hiddenPlacement).filter(placement => {
        if (placement.side !== side || state[side].some(group => group.id === placement.groupId) || seen.has(placement.groupId)) return false;
        seen.add(placement.groupId); return true;
    });
}
export function moveDockView(state: DockState, id: string, target: DockMoveTarget): DockState {
    if (!sides.includes(target.side) || ![...state.left, ...state.right].some(group => group.views.includes(id)) && !state.hidden.includes(id)) return clone(state);
    const next = clone(state), position = target.position ?? "center";
    // A split against the sole member's own group has no effect.
    const originalTarget = state[target.side].find(group => group.id === target.groupId);
    if (originalTarget?.views.length === 1 && originalTarget.views[0] === id) return next;
    removeView(next, id); delete next.hiddenPlacement[id];
    const groups = next[target.side], anchor = groups.find(group => group.id === target.groupId);
    if (position === "center" && anchor) {
        anchor.views.push(id); anchor.active = id; anchor.collapsed = false;
    } else if (groups.length >= maxGroups) {
        const group = anchor ?? groups.at(-1)!; group.views.push(id); group.active = id; group.collapsed = false;
    } else {
        const group: DockGroup = { id: freshGroupId(next, `${target.side}-group`), views: [id], active: id, weight: 1, collapsed: false };
        const at = anchor ? groups.indexOf(anchor) + (position === "after" ? 1 : 0) : groups.length;
        groups.splice(at, 0, group);
    }
    next[`${target.side}Hidden`] = false;
    next.maximized = null;
    return next;
}
export function hideDockView(state: DockState, id: string): DockState {
    const next = clone(state);
    for (const side of sides) {
        const groupIndex = state[side].findIndex(group => group.views.includes(id));
        if (groupIndex < 0) continue;
        const group = state[side][groupIndex];
        const hiddenTabs = Object.values(state.hiddenPlacement).filter(placement => placement.side === side && placement.groupId === group.id);
        // Remaining visible tabs may have resized the group since earlier tabs
        // were hidden. Preserve its latest geometry for any later reveal order.
        for (const placement of Object.values(next.hiddenPlacement)) if (placement.side === side && placement.groupId === group.id) placement.weight = group.weight;
        next.hiddenPlacement[id] = { side, groupId: group.id,
            groupIndex: hiddenTabs[0]?.groupIndex ?? logicalIndex(groupIndex, missingGroupPlacements(state, side).map(placement => placement.groupIndex)),
            index: logicalIndex(group.views.indexOf(id), hiddenTabs.map(placement => placement.index)), weight: group.weight };
        removeView(next, id); next.hidden.push(id);
        break;
    }
    return next;
}
export function revealDockView(state: DockState, id: string, views: readonly DockViewDefinition[]): DockState {
    const next = clone(state), definition = definitions(views).find(view => view.id === id);
    if (!definition) return next;
    for (const side of sides) {
        const group = next[side].find(group => group.views.includes(id));
        if (group) {
            group.active = id; group.collapsed = false; next[`${side}Hidden`] = false;
            if (next.maximized && next.maximized !== group.id) next.maximized = null;
            return next;
        }
    }
    const placement = next.hiddenPlacement[id];
    next.hidden = next.hidden.filter(view => view !== id); delete next.hiddenPlacement[id];
    if (placement) {
        const groups = next[placement.side];
        let group = groups.find(item => item.id === placement.groupId);
        if (!group && groups.length < maxGroups) {
            group = { id: freshGroupId(next, placement.groupId), views: [], active: id, weight: placement.weight ?? 1, collapsed: false };
            const hiddenBefore = missingGroupPlacements(next, placement.side).filter(item => item.groupIndex < placement.groupIndex).length;
            groups.splice(Math.min(Math.max(0, placement.groupIndex - hiddenBefore), groups.length), 0, group);
        }
        if (group) {
            const hiddenBefore = Object.values(next.hiddenPlacement).filter(item => item.side === placement.side && item.groupId === placement.groupId && item.index < placement.index).length;
            group.views.splice(Math.min(Math.max(0, placement.index - hiddenBefore), group.views.length), 0, id); group.active = id; group.collapsed = false;
            next[`${placement.side}Hidden`] = false; next.maximized = null;
            return next;
        }
    }
    appendDefault(next, definition);
    const group = next[definition.side].find(item => item.views.includes(id))!;
    group.active = id; group.collapsed = false; next[`${definition.side}Hidden`] = false; next.maximized = null;
    return next;
}
/** Transfer a fraction of this adjacent pair's total weight, preserving the total. */
export function resizeDockGroups(state: DockState, side: DockSide, firstIndex: number, deltaFraction: number): DockState {
    const next = clone(state);
    if (!sides.includes(side) || !Number.isInteger(firstIndex) || firstIndex < 0 || !Number.isFinite(deltaFraction)) return next;
    const first = next[side][firstIndex], second = next[side][firstIndex + 1];
    if (!first || !second || first.collapsed || second.collapsed) return next;
    const total = first.weight + second.weight;
    const min = Math.max(.1, total * .1, total - 10), max = Math.min(10, total * .9, total - .1);
    first.weight = Math.min(max, Math.max(min, first.weight + total * deltaFraction));
    second.weight = total - first.weight;
    return next;
}
export function dockContextKey(context: string): string { return `weftext-workspace-dock:v1:${encodeURIComponent(context)}`; }
