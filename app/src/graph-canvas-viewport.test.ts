import test from "node:test";
import assert from "node:assert/strict";
import { canvasBoundsVisible, fitCanvasCamera, headingCardWidth, revealCanvasCamera, scaleCanvasCamera } from "./graph-canvas-viewport";

test("264/420 sidebar cards retain CSS text scale and both branch controls fit", () => {
    for (const width of [248, 404, 1200]) {
        const viewport = { width, height: 318 }, box = { x: 150, y: 55, halfWidth: headingCardWidth(viewport) / 2, halfHeight: 23 };
        for (const direction of ["horizontal", "vertical"] as const) {
            const camera = revealCanvasCamera({ x: 0, y: 0, scale: .2 }, box, viewport, direction);
            assert.equal(camera.scale, 1); assert.equal(canvasBoundsVisible(camera, box, viewport), true);
        }
    }
});
test("search reveals real targets readably, preserving an explicit zoom above 100%", () => {
    const viewport = { width: 248, height: 318 }, box = { x: 1800, y: 700, halfWidth: 110, halfHeight: 23 };
    const camera = revealCanvasCamera({ x: -20, y: 5, scale: .3 }, box, viewport);
    assert.equal(camera.scale, 1); assert.equal(canvasBoundsVisible(camera, box, viewport), true);
    assert.equal(revealCanvasCamera({ ...camera, scale: 2 }, { ...box, halfWidth: 19 }, viewport).scale, 2);
    assert.deepEqual(revealCanvasCamera(camera, box, viewport), camera);
    assert.equal(revealCanvasCamera({ ...camera, scale: .3 }, box, viewport, false, false).scale, .3, "ordinary focus preserves an explicit overview zoom");
});
test("100% is one CSS pixel per world unit and retains the anchor world point", () => {
    const before = { x: -400, y: 120, scale: .25 }, anchor = { x: 124, y: 159 };
    const after = scaleCanvasCamera(before, 1, anchor);
    assert.equal(after.scale, 1); assert.equal((anchor.x - before.x) / before.scale, (anchor.x - after.x) / after.scale);
    assert.equal((anchor.y - before.y) / before.scale, (anchor.y - after.y) / after.scale);
});
test("only explicit Fit shrinks a distant tree overview into the actual viewport", () => {
    const viewport = { width: 248, height: 318 }, boxes = [{ x: 150, y: 55, halfWidth: 112, halfHeight: 23 }, { x: 60000, y: 800, halfWidth: 112, halfHeight: 23 }];
    const camera = fitCanvasCamera(boxes, viewport);
    assert.ok(camera.scale < .1);
    for (const box of boxes) assert.equal(canvasBoundsVisible(camera, box, viewport), true);
});
