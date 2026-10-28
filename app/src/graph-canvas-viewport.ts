import type { Camera, Point } from "./relationship-graph-state";

export interface CanvasViewport { width: number; height: number }
export interface CanvasBounds extends Point { halfWidth: number; halfHeight: number }
export function headingCardWidth(viewport: CanvasViewport): number { return Math.max(180, Math.min(256, viewport.width - 24)); }
/** Camera units are CSS pixels; changing scale retains the point below the anchor. */
export function scaleCanvasCamera(camera: Camera, scale: number, anchor: Point): Camera {
    scale = Math.max(.001, Math.min(4, scale));
    const ratio = scale / camera.scale;
    return { scale, x: anchor.x - (anchor.x - camera.x) * ratio, y: anchor.y - (anchor.y - camera.y) * ratio };
}
/** Explicit overview only. Ordinary opening/navigation must not shrink text to fit. */
export function fitCanvasCamera(boxes: CanvasBounds[], viewport: CanvasViewport): Camera {
    if (!boxes.length) return { x: 0, y: 0, scale: 1 };
    const left = Math.min(...boxes.map(box => box.x - box.halfWidth)), right = Math.max(...boxes.map(box => box.x + box.halfWidth));
    const top = Math.min(...boxes.map(box => box.y - box.halfHeight)), bottom = Math.max(...boxes.map(box => box.y + box.halfHeight));
    const scale = Math.max(.001, Math.min(1.6, (viewport.width - 24) / Math.max(1, right - left), (viewport.height - 24) / Math.max(1, bottom - top)));
    return { scale, x: (viewport.width - (right + left) * scale) / 2, y: (viewport.height - (bottom + top) * scale) / 2 };
}
export function canvasBoundsVisible(camera: Camera, box: CanvasBounds, viewport: CanvasViewport): boolean {
    const x = box.x * camera.scale + camera.x, y = box.y * camera.scale + camera.y;
    return x - box.halfWidth * camera.scale >= 0 && x + box.halfWidth * camera.scale <= viewport.width
        && y - box.halfHeight * camera.scale >= 0 && y + box.halfHeight * camera.scale <= viewport.height;
}
/** Keep readable text; translate only enough to reveal a body or branch control. */
export function revealCanvasCamera(camera: Camera, box: CanvasBounds, viewport: CanvasViewport, root: false | "horizontal" | "vertical" = false, readable = true): Camera {
    const scale = root || readable ? Math.max(1, camera.scale) : camera.scale;
    if (root) return { scale, x: (root === "horizontal" ? 12 + box.halfWidth * scale : viewport.width / 2) - box.x * scale, y: (root === "vertical" ? 12 + box.halfHeight * scale : viewport.height / 2) - box.y * scale };
    const nearest = (center: number, half: number, size: number) => half * 2 > size - 24 ? size / 2 : Math.max(12 + half, Math.min(size - 12 - half, center));
    const x = box.x * scale + camera.x, y = box.y * scale + camera.y;
    return { scale, x: camera.x + nearest(x, box.halfWidth * scale, viewport.width) - x, y: camera.y + nearest(y, box.halfHeight * scale, viewport.height) - y };
}
