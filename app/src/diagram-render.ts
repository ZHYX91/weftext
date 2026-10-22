import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Bundled Mermaid runs on the editing client, without a rendering service. */
import { validateDiagramSource, validatePng, type DiagramImage } from "./diagram-state";
let rendering: Promise<unknown> = Promise.resolve();
const localFont = '"Microsoft YaHei", "Noto Sans CJK SC", "Noto Sans", sans-serif';
async function render(source: string, requestedWidth: number): Promise<DiagramImage> {
    validateDiagramSource(source);
    if (!Number.isSafeInteger(requestedWidth) || requestedWidth < 320 || requestedWidth > 2048)
        throw new UiError(uiMessage("diagramRender.widthOutOfRange"));
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({ startOnLoad: false, securityLevel: "strict", htmlLabels: false, fontFamily: localFont, maxTextSize: 32768, maxEdges: 250, suppressErrorRendering: true, theme: "default", secure: ["secure", "securityLevel", "startOnLoad", "maxTextSize", "maxEdges", "fontFamily", "htmlLabels", "theme", "themeCSS"] });
    await document.fonts.ready;
    const host = document.createElement("div");
    host.style.cssText = "position:fixed;left:-100000px;top:0;visibility:hidden;width:2048px;pointer-events:none";
    host.setAttribute("aria-hidden", "true");
    document.body.append(host);
    try {
        const { svg } = await mermaid.render(`weftext-diagram-${crypto.randomUUID()}`, source, host);
        const parsed = new DOMParser().parseFromString(svg, "image/svg+xml");
        const root = parsed.documentElement;
        if (root.localName !== "svg" || parsed.querySelector("parsererror,script,foreignObject,image,iframe,object,embed"))
            throw new UiError(uiMessage("diagramRender.pngContentUnsupported"));
        for (const element of parsed.querySelectorAll("*"))
            for (const attribute of element.attributes) {
                if (/^on/i.test(attribute.name) || /(?:href|src)$/i.test(attribute.localName) && attribute.value && !attribute.value.startsWith("#"))
                    throw new UiError(uiMessage("diagramRender.externalResourcesDisallowed"));
            }
        if (/@import|url\(\s*["']?(?!#)[^)]+\)/i.test(root.textContent ?? ""))
            throw new UiError(uiMessage("diagramRender.styleNeedsExternalResources"));
        const box = root.getAttribute("viewBox")?.trim().split(/[ ,]+/).map(Number);
        if (!box || box.length !== 4 || !box.every(Number.isFinite) || box[2] <= 0 || box[3] <= 0)
            throw new UiError(uiMessage("diagramRender.dimensionsMissing"));
        const width = requestedWidth, height = Math.max(1, Math.ceil(width * box[3] / box[2]));
        if (height > 4096 || width * height > 8388608)
            throw new UiError(uiMessage("diagramRender.imageTooTall"));
        root.setAttribute("width", String(width));
        root.setAttribute("height", String(height));
        root.setAttribute("style", `font-family:${localFont}`);
        const blob = new Blob([new XMLSerializer().serializeToString(root)], { type: "image/svg+xml" });
        const url = URL.createObjectURL(blob);
        try {
            const image = new Image();
            image.src = url;
            await image.decode();
            const canvas = document.createElement("canvas");
            canvas.width = width;
            canvas.height = height;
            const context = canvas.getContext("2d");
            if (!context)
                throw new UiError(uiMessage("diagramRender.pngUnavailable"));
            context.fillStyle = "white";
            context.fillRect(0, 0, width, height);
            context.drawImage(image, 0, 0, width, height);
            const pngBlob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(value => value ? resolve(value) : reject(new UiError(uiMessage("diagramRender.pngFailed"))), "image/png"));
            const png = new Uint8Array(await pngBlob.arrayBuffer());
            validatePng(png, width, height);
            return { source, png, width, height };
        }
        finally {
            URL.revokeObjectURL(url);
        }
    }
    finally {
        host.remove();
    }
}
/** Mermaid owns mutable global state; serialize independent dialogs. */
export function renderDiagramPng(source: string, width: number): Promise<DiagramImage> {
    const next = rendering.catch(() => { }).then(() => render(source, width));
    rendering = next;
    return next;
}
