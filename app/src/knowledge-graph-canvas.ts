import { message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { type Camera, type Point } from "./relationship-graph-state";
import { canvasBoundsVisible, fitCanvasCamera, headingCardWidth, revealCanvasCamera, scaleCanvasCamera, type CanvasBounds, type CanvasViewport } from "./graph-canvas-viewport";
export interface CanvasNode {
    key: string;
    title: UiText;
    heading?: { number: UiText; childCount: number; expanded: boolean; label: UiText; toggle?: () => void };
}
export interface CanvasEdge {
    key: string;
    source: string;
    target: string;
    directed: boolean;
    label: UiText;
}
export interface CanvasState {
    positions: Map<string, Point>;
    camera: Camera;
    viewport?: CanvasViewport;
}
const canvasDisposers = new WeakMap<Element, () => void>();
/** The view owner calls this before replacing or retiring its canvas DOM. */
export function disposeGraphCanvasTree(root: ParentNode): void {
    for (const canvas of root.querySelectorAll(".knowledge-graph-canvas")) canvasDisposers.get(canvas)?.();
    canvasDisposers.get(root as Element)?.();
}
function svg<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, UiText>): SVGElementTagNameMap[K] {
    const node = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const [key, value] of Object.entries(attrs)) {
        if (key === "aria-label" || key === "title")
            setUiAttr(node, key, value);
        else
            node.setAttribute(key, uiTextValue(value));
    }
    return node;
}
let canvasSerial = 0;
/** Display state only. Pointer and keyboard handlers have no authoring host. */
export function graphCanvas(nodes: CanvasNode[], edges: CanvasEdge[], state: CanvasState, select: (key: string, edge: boolean) => void, open: (key: string) => void, selected: string, label: UiText, options: { revealKey?: string; centerReveal?: boolean; fit?: boolean; readableRootKey?: string; direction?: "horizontal" | "vertical" } = {}): HTMLElement {
    const nodeByKey = new Map(nodes.map(node => [node.key, node]));
    const host = document.createElement("div");
    host.className = "knowledge-graph-canvas";
    const controls = document.createElement("div");
    controls.className = "knowledge-graph-controls";
    const drawing = svg("svg", { viewBox: "0 0 700 420", role: "group", "aria-label": label, tabindex: "0" });
    // Explicit sizing prevents intrinsic SVG aspect-ratio feedback and generic icon CSS.
    drawing.style.width = "100%"; drawing.style.height = "320px"; drawing.style.minHeight = "240px";
    let viewport = state.viewport ?? { width: 700, height: 420 }, disposed = false, measured = false;
    let pendingRoot = options.readableRootKey ?? (!state.viewport && nodes[0]?.heading ? nodes[0].key : undefined), pendingReveal = options.revealKey, pendingFit = options.fit === true;
    const events = new AbortController();
    const headingCards: { node: CanvasNode; rect: SVGRectElement; bodyClip: SVGRectElement; numberClip: SVGRectElement; number: SVGTextElement; branch: SVGGElement }[] = [];
    const markerId = `knowledge-arrow-${++canvasSerial}`, defs = svg("defs", {}), marker = svg("marker", { id: markerId, markerWidth: "8", markerHeight: "8", refX: "8", refY: "4", orient: "auto", markerUnits: "strokeWidth" });
    marker.append(svg("path", { d: "M0,0 L8,4 L0,8 z", fill: "currentColor" }));
    defs.append(marker);
    drawing.append(defs);
    const world = svg("g", {});
    drawing.append(world);
    const transforms = () => { if (!disposed) world.setAttribute("transform", `translate(${state.camera.x} ${state.camera.y}) scale(${state.camera.scale})`); };
    const bounds = (key: string, branch = false): CanvasBounds | undefined => {
        const point = state.positions.get(key), item = nodeByKey.get(key);
        return !point || !item ? undefined : branch && item.heading ? { x: point.x + headingCardWidth(viewport) / 2 - 21, y: point.y, halfWidth: 19, halfHeight: 19 } : { ...point, halfWidth: item.heading ? headingCardWidth(viewport) / 2 : 72, halfHeight: 23 };
    };
    const reveal = (key: string, branch = false, root: false | "horizontal" | "vertical" = false, readable = false): void => { const box = bounds(key, branch); if (!disposed && box) { state.camera = revealCanvasCamera(state.camera, box, viewport, root, readable); transforms(); } };
    const center = () => ({ x: viewport.width / 2, y: viewport.height / 2 });
    const button = (text: UiText, run: () => void) => { const b = document.createElement("button"); b.type = "button"; setUiText(b, text); b.onclick = () => { if (!disposed) run(); }; controls.append(b); };
    button(message("graph.zoomIn"), () => { state.camera = scaleCanvasCamera(state.camera, state.camera.scale * 1.25, center()); transforms(); });
    button(message("graph.zoomOut"), () => { state.camera = scaleCanvasCamera(state.camera, state.camera.scale * .8, center()); transforms(); });
    button("100%", () => { state.camera = scaleCanvasCamera(state.camera, 1, center()); if (nodes[0]?.heading) reveal(nodeByKey.has(selected) ? selected : nodes[0].key); transforms(); });
    const fit = (): void => { state.camera = fitCanvasCamera(nodes.map(node => bounds(node.key)).filter((box): box is CanvasBounds => box !== undefined), viewport); };
    button(message("graph.fitViewport"), () => { fit(); transforms(); });
    for (const [title, dx, dy] of [[message("graph.panLeft"), 60, 0], [message("graph.panRight"), -60, 0], [message("graph.panUp"), 0, 60], [message("graph.panDown"), 0, -60]] as const)
        button(title, () => { state.camera.x += dx; state.camera.y += dy; transforms(); });
    const nodeElements = new Map<string, SVGGElement>(), edgeElements: {
        edge: CanvasEdge;
        path: SVGPathElement;
        offset: number;
    }[] = [], groups = new Map<string, number>();
    for (const edge of edges) {
        const pair = [edge.source, edge.target].sort().join("\n"), index = groups.get(pair) ?? 0;
        groups.set(pair, index + 1);
        const path = svg("path", { class: `knowledge-graph-edge${selected === edge.key ? " is-selected" : ""}`, "data-graph-key": edge.key, fill: "none", stroke: "currentColor", "stroke-width": "2", tabindex: "0", role: "button", "aria-label": edge.label });
        if (edge.directed)
            path.setAttribute("marker-end", `url(#${markerId})`);
        path.onclick = event => { event.stopPropagation(); select(edge.key, true); };
        path.onkeydown = event => {
            if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                select(edge.key, true);
            }
        };
        const title = svg("title", {});
        setUiText(title, edge.label);
        path.append(title);
        world.append(path);
        edgeElements.push({ edge, path, offset: index === 0 ? 0 : (index % 2 ? 1 : -1) * Math.ceil(index / 2) * 28 });
    }
    const redraw = () => {
        for (const [key, node] of nodeElements) {
            const p = state.positions.get(key)!;
            node.setAttribute("transform", `translate(${p.x} ${p.y})`);
        }
        for (const { edge, path, offset } of edgeElements) {
            const a = state.positions.get(edge.source), b = state.positions.get(edge.target);
            if (!a || !b)
                continue;
            if (edge.source === edge.target)
                path.setAttribute("d", `M${a.x - 40},${a.y - 26} C${a.x - 75 - offset},${a.y - 90 - offset} ${a.x + 75 + offset},${a.y - 90 - offset} ${a.x + 40},${a.y - 26}`);
            else {
                const dx = b.x - a.x, dy = b.y - a.y, length = Math.max(1, Math.hypot(dx, dy)), border = Math.min(.48, (nodeByKey.get(edge.target)?.heading ? headingCardWidth(viewport) / 2 + 6 : 78) / Math.max(.001, Math.abs(dx)), 28 / Math.max(.001, Math.abs(dy))), end = { x: b.x - dx * border, y: b.y - dy * border };
                path.setAttribute("d", `M${a.x},${a.y} Q${(a.x + b.x) / 2 - dy / length * offset},${(a.y + b.y) / 2 + dx / length * offset} ${end.x},${end.y}`);
            }
        }
    };
    let drag: {
        pointer: number;
        x: number;
        y: number;
        node?: string;
        origin: Point;
        moved: boolean;
    } | undefined;
    let lastTap: {
        key: string;
        time: number;
    } | undefined;
    const coordinates = (event: PointerEvent | WheelEvent) => {
        const matrix = drawing.getScreenCTM();
        if (matrix) {
            const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
            return { x: point.x, y: point.y };
        }
        const rect = drawing.getBoundingClientRect();
        return { x: (event.clientX - rect.left) * viewport.width / Math.max(rect.width, 1), y: (event.clientY - rect.top) * viewport.height / Math.max(rect.height, 1) };
    };
    for (const node of nodes) {
        const group = svg("g", { class: `knowledge-graph-node${node.heading ? " heading-map-card" : ""}${selected === node.key ? " is-selected" : ""}`, "data-graph-key": node.key, tabindex: node.heading ? "-1" : "0", role: node.heading ? "group" : "button", "aria-label": () => message("graph.nodeKeyboardInstructions", { p0: uiTextValue(node.title) }) });
        group.append(svg("rect", { x: node.heading ? "-128" : "-72", y: "-23", width: node.heading ? "256" : "144", height: "46", rx: "10" }));
        const text = svg("text", { "text-anchor": "middle", dy: ".35em" });
        setUiText(text, () => { const title = uiTextValue(node.title), graphemes = Array.from(new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(title), item => item.segment); return graphemes.length > 14 ? `${graphemes.slice(0, 13).join("")}…` : title; });
        const title = svg("title", {});
        setUiText(title, node.title);
        group.append(text, title);
        if (node.heading) {
            const body = svg("g", { role: "button", tabindex: "0", "aria-label": () => `${uiTextValue(node.heading!.number)} ${uiTextValue(node.title)}`, "data-outline-control": `map-item:${node.key}` });
            body.append(...Array.from(group.childNodes)); group.append(body); body.onfocus = () => reveal(node.key);
            const clipId = `heading-card-${canvasSerial}-${nodeElements.size}`, bodyClip = svg("clipPath", { id: `${clipId}-body` }), numberClip = svg("clipPath", { id: `${clipId}-number` });
            bodyClip.append(svg("rect", { x: "-79", y: "-22", width: "159", height: "44" }));
            numberClip.append(svg("rect", { x: "-124", y: "-22", width: "40", height: "44" }));
            defs.append(bodyClip, numberClip); text.setAttribute("clip-path", `url(#${clipId}-body)`);
            const number = svg("text", { x: "-104", dy: ".35em", "text-anchor": "middle", class: "heading-map-number", "aria-hidden": "true" });
            number.setAttribute("clip-path", `url(#${clipId}-number)`);
            setUiText(number, () => {
                const full = uiTextValue(node.heading!.number), parts = Array.from(new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(full), item => item.segment);
                if (parts.reduce((length, part) => length + (/^[\x00-\x7f]+$/.test(part) ? .6 : 1), 0) <= 3.1) return full;
                let units = 0, visible = "";
                for (const part of parts) { units += /^[\x00-\x7f]+$/.test(part) ? .6 : 1; if (units > 2.1) break; visible += part; }
                return `${visible}…`;
            });
            const numberTitle = svg("title", {}); setUiText(numberTitle, node.heading.number); number.append(numberTitle);
            body.append(number);
            const branch = svg("g", { class: "heading-map-branch", transform: "translate(107 0)", role: "button", tabindex: node.heading.toggle ? "0" : "-1", "aria-label": node.heading.label, "aria-disabled": String(!node.heading.toggle), "data-heading-branch": node.key, "data-outline-control": `map-fold:${node.key}` });
            if (node.heading.childCount) branch.setAttribute("aria-expanded", String(node.heading.expanded));
            branch.append(svg("rect", { x: "-19", y: "-19", width: "38", height: "38", rx: "6" }));
            const count = svg("text", { "text-anchor": "middle", dy: ".35em", "aria-hidden": "true" });
            setUiText(count, `${node.heading.childCount ? node.heading.expanded ? "−" : "+" : ""}${node.heading.childCount}`);
            branch.append(count);
            branch.onpointerdown = event => event.stopPropagation();
            branch.onclick = event => { event.preventDefault(); event.stopPropagation(); node.heading?.toggle?.(); };
            branch.onkeydown = event => { if (event.isComposing || event.keyCode === 229) return; if (event.key === "Enter" || event.key === " ") { event.preventDefault(); event.stopPropagation(); node.heading?.toggle?.(); } };
            branch.onfocus = () => reveal(node.key, true);
            group.append(branch);
            headingCards.push({ node, rect: body.querySelector("rect")!, bodyClip: bodyClip.querySelector("rect")!, numberClip: numberClip.querySelector("rect")!, number, branch });
        }
        group.onclick = event => { event.stopPropagation(); };
        group.ondblclick = event => { event.stopPropagation(); event.preventDefault(); };
        group.onkeydown = event => {
            if (event.isComposing || event.keyCode === 229) return;
            if (event.key === "Enter") {
                event.preventDefault();
                open(node.key);
            }
            else if (event.key === " ") {
                event.preventDefault();
                select(node.key, false);
            }
            else if (event.key === "Escape") {
                event.preventDefault();
                select("", false);
            }
            else if (event.shiftKey && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
                event.preventDefault();
                const position = state.positions.get(node.key)!;
                position.x += event.key === "ArrowLeft" ? -20 : event.key === "ArrowRight" ? 20 : 0;
                position.y += event.key === "ArrowUp" ? -20 : event.key === "ArrowDown" ? 20 : 0;
                redraw();
            }
        };
        group.onpointerdown = event => {
            if (event.button !== 0)
                return;
            const point = coordinates(event);
            drag = { pointer: event.pointerId, ...point, node: node.key, origin: { ...state.positions.get(node.key)! }, moved: false };
            drawing.setPointerCapture(event.pointerId);
            event.stopPropagation();
        };
        world.append(group);
        nodeElements.set(node.key, group);
    }
    drawing.onpointerdown = event => {
        if (event.button !== 0 || (event.target !== drawing && event.target !== world))
            return;
        const point = coordinates(event);
        drag = { pointer: event.pointerId, ...point, origin: { x: state.camera.x, y: state.camera.y }, moved: false };
        drawing.setPointerCapture(event.pointerId);
    };
    drawing.onpointermove = event => {
        if (!drag || drag.pointer !== event.pointerId)
            return;
        const point = coordinates(event), dx = point.x - drag.x, dy = point.y - drag.y;
        if (Math.hypot(dx, dy) > 4)
            drag.moved = true;
        if (drag.node) {
            state.positions.set(drag.node, { x: drag.origin.x + dx / state.camera.scale, y: drag.origin.y + dy / state.camera.scale });
            redraw();
        }
        else {
            state.camera.x = drag.origin.x + dx;
            state.camera.y = drag.origin.y + dy;
            transforms();
        }
    };
    drawing.onpointerup = event => {
        if (!drag || drag.pointer !== event.pointerId)
            return;
        const previous = drag;
        drawing.releasePointerCapture(event.pointerId);
        drag = undefined;
        if (!previous.moved) {
            select(previous.node ?? "", false);
            if (previous.node) {
                const now = performance.now();
                if (lastTap?.key === previous.node && now - lastTap.time < 400) {
                    lastTap = undefined;
                    open(previous.node);
                }
                else
                    lastTap = { key: previous.node, time: now };
            }
        }
        else
            lastTap = undefined;
    };
    drawing.onpointercancel = () => { drag = undefined; };
    drawing.onkeydown = event => {
        if (event.target !== drawing)
            return;
        if (event.key === "Escape") {
            event.preventDefault();
            select("", false);
        }
        else if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
            event.preventDefault();
            state.camera.x += event.key === "ArrowLeft" ? 40 : event.key === "ArrowRight" ? -40 : 0;
            state.camera.y += event.key === "ArrowUp" ? 40 : event.key === "ArrowDown" ? -40 : 0;
            transforms();
        }
    };
    drawing.addEventListener("wheel", event => { event.preventDefault(); state.camera = scaleCanvasCamera(state.camera, state.camera.scale * (event.deltaY < 0 ? 1.1 : 1 / 1.1), coordinates(event)); transforms(); }, { passive: false, signal: events.signal });
    const syncViewport = (): void => {
        if (disposed || !drawing.isConnected) return;
        const style = getComputedStyle(drawing), box = drawing.getBoundingClientRect();
        const edge = (a: string, b: string) => (parseFloat(a) || 0) + (parseFloat(b) || 0);
        const borderX = edge(style.borderLeftWidth, style.borderRightWidth), borderY = edge(style.borderTopWidth, style.borderBottomWidth);
        const width = box.width - borderX, height = box.height - borderY;
        if (width <= 0 || height <= 0) return; // Hidden/parked Dock views retain their camera and observer.
        const oldRoot = nodes[0] && bounds(nodes[0].key), changed = width !== viewport.width || height !== viewport.height, knownViewport = measured || state.viewport !== undefined, retainRoot = knownViewport && changed && oldRoot && state.camera.scale >= 1 && canvasBoundsVisible(state.camera, oldRoot, viewport);
        if (knownViewport && changed) { state.camera.x += (width - viewport.width) / 2; state.camera.y += (height - viewport.height) / 2; }
        viewport = { width, height }; state.viewport = viewport;
        drawing.setAttribute("viewBox", `0 0 ${width} ${height}`);
        for (const card of headingCards) {
            const half = headingCardWidth(viewport) / 2;
            card.rect.setAttribute("x", String(-half)); card.rect.setAttribute("width", String(half * 2));
            card.bodyClip.setAttribute("x", String(-half + 49)); card.bodyClip.setAttribute("width", String(half * 2 - 97));
            card.numberClip.setAttribute("x", String(-half + 4)); card.number.setAttribute("x", String(-half + 24));
            card.branch.setAttribute("transform", `translate(${half - 21} 0)`);
        }
        redraw();
        if (pendingFit) { fit(); pendingFit = false; }
        if (pendingRoot) { reveal(pendingRoot, false, options.direction ?? "horizontal"); pendingRoot = undefined; }
        else if (retainRoot && nodes[0]?.heading) reveal(nodes[0].key);
        if (pendingReveal) {
            const point = state.positions.get(pendingReveal);
            if (options.centerReveal && point) { state.camera.scale = Math.max(1, state.camera.scale); state.camera.x = width / 2 - point.x * state.camera.scale; state.camera.y = height / 2 - point.y * state.camera.scale; }
            else reveal(pendingReveal, false, false, true);
            pendingReveal = undefined;
        }
        measured = true; transforms();
    };
    const observer = new ResizeObserver(syncViewport);
    observer.observe(drawing);
    canvasDisposers.set(host, () => {
        if (disposed) return;
        disposed = true; observer.disconnect(); events.abort();
        if (drag && drawing.hasPointerCapture(drag.pointer)) { try { drawing.releasePointerCapture(drag.pointer); } catch { /* Detached SVGs may already have released the capture. */ } }
        drag = undefined; lastTap = undefined;
        for (const element of host.querySelectorAll<HTMLElement | SVGElement>("*")) {
            element.onclick = null; element.ondblclick = null; element.onkeydown = null; element.onfocus = null;
            element.onpointerdown = null; element.onpointermove = null; element.onpointerup = null; element.onpointercancel = null;
        }
        canvasDisposers.delete(host);
    });
    redraw();
    transforms();
    queueMicrotask(syncViewport);
    host.append(controls, drawing);
    return host;
}
