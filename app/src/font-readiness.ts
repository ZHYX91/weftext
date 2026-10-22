import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Actual same-origin font loading; failures never change native source or drafts. */
export const CHINESE_FONT_SAMPLE = "简体中文繁體漢字，。！？总价單價數量";
export interface RenderedFontRequest {
    family: string;
    weight?: number;
    style?: "normal" | "italic" | "oblique";
    text?: string;
}
export interface DeliveredFont {
    file: string;
    family: string;
    role: string;
    weight: number;
    style?: string;
    url?: string;
    browser?: {
        file: string;
        url: string;
        family: string;
        weight: number;
        style: string;
    };
    inspection?: {
        hasMathTable?: boolean;
    };
}
export interface FontDeliveryManifest {
    fonts: DeliveredFont[];
}
interface LoadedFace {
    family: string;
    status: string;
}
export interface FontReadinessHost {
    fonts?: {
        load(font: string, text?: string): Promise<readonly LoadedFace[]>;
        check(font: string, text?: string): boolean;
    };
    readManifest(): Promise<FontDeliveryManifest>;
    timeoutMs?: number;
}
export class OfflineFontError extends UiError {
    readonly code = "offlineFontsUnavailable";
    constructor(message: UiText, readonly families: string[] = []) { super(message); this.name = "OfflineFontError"; }
}
const defaultFonts: RenderedFontRequest[] = [
    { family: "Noto Sans CJK SC", text: CHINESE_FONT_SAMPLE },
    { family: "New Computer Modern", text: "Weftext ABCxyz 0123" },
    { family: "Noto Serif CJK SC", text: CHINESE_FONT_SAMPLE },
    { family: "Noto Sans Mono CJK SC", text: `ABC 0123 ${CHINESE_FONT_SAMPLE}` },
    { family: "DejaVu Sans Mono", text: "ABC 0123 []{}()" },
    { family: "New Computer Modern Math", text: "∑∫√()abc0123" },
];
const normalizedFamily = (family: string): string => family.trim().replace(/^(["'])(.*)\1$/, "$2").toLowerCase();
const cssFamily = (family: string): string => `"${family.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
async function bounded<T>(promise: Promise<T>, timeoutMs: number, message: UiText): Promise<T> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
        return await Promise.race([promise, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new OfflineFontError(message)), timeoutMs); })]);
    }
    finally {
        if (timer !== undefined)
            clearTimeout(timer);
    }
}
/** Host injection permits meaningful transport/font-result tests without a browser. */
export class OfflineFontLoader {
    private manifest: Promise<FontDeliveryManifest> | null = null;
    private readonly inFlight = new Map<string, Promise<void>>();
    constructor(readonly host: FontReadinessHost) { }
    async ensureDefaults(): Promise<void> { await this.ensure(defaultFonts); }
    async ensure(requests: readonly RenderedFontRequest[]): Promise<void> {
        if (!this.host.fonts || typeof this.host.fonts.load !== "function" || typeof this.host.fonts.check !== "function")
            throw new OfflineFontError(uiMessage("fontReadiness.fontApiUnavailable"));
        const manifest = await this.readManifest();
        const results = await Promise.allSettled(requests.map(request => this.load(request, manifest)));
        const errors = results.filter((result): result is PromiseRejectedResult => result.status === "rejected").map(result => result.reason);
        if (errors.length) {
            const families = errors.flatMap(error => error instanceof OfflineFontError ? error.families : []);
            throw new OfflineFontError(joinUiText(errors.map(error => errorUiText(error)), "\n"), [...new Set(families)]);
        }
    }
    private async readManifest(): Promise<FontDeliveryManifest> {
        if (!this.manifest) {
            this.manifest = bounded(this.host.readManifest(), this.host.timeoutMs ?? 15000, uiMessage("fontReadiness.manifestTimeout")).then(manifest => {
                if (!manifest || !Array.isArray(manifest.fonts) || !manifest.fonts.length || !manifest.fonts.every(entry => entry && typeof entry.family === "string" && typeof entry.file === "string" && typeof entry.role === "string" && typeof entry.weight === "number" && Number.isFinite(entry.weight) && entry.weight > 0))
                    throw new OfflineFontError(uiMessage("fontReadiness.manifestInvalid"));
                return manifest;
            });
        }
        try {
            return await this.manifest;
        }
        catch (error) {
            this.manifest = null;
            throw error instanceof OfflineFontError ? error : new OfflineFontError(displayMessage("fontReadiness.manifestReadFailed", { p0: uiDisplayParameter(errorUiText(error)) }));
        }
    }
    private load(request: RenderedFontRequest, manifest: FontDeliveryManifest): Promise<void> {
        const family = request.family.trim();
        const weight = request.weight ?? 400;
        const style = request.style ?? "normal";
        if (!family || /[\r\n\0]/.test(family) || !Number.isFinite(weight) || weight < 1 || weight > 1000 || !["normal", "italic", "oblique"].includes(style))
            return Promise.reject(new OfflineFontError(uiMessage("fontReadiness.fontParametersUnconfirmed"), [family]));
        const entries = manifest.fonts.filter(entry => typeof entry.family === "string" && normalizedFamily(entry.family) === normalizedFamily(family));
        if (!entries.length)
            return Promise.reject(new OfflineFontError(displayMessage("fontReadiness.fontNotDelivered", { p0: uiDisplayParameter(family) }), [family]));
        for (const entry of entries) {
            if (typeof entry.file !== "string" || !/^[A-Za-z0-9_-]+\.(?:otf|ttf)$/.test(entry.file) || (entry.url !== undefined && entry.url !== `/fonts/${entry.file}`))
                return Promise.reject(new OfflineFontError(displayMessage("fontReadiness.fontAddressMismatch", { p0: uiDisplayParameter(family) }), [family]));
            if (entry.browser && (!/^[A-Za-z0-9_-]+\.woff2$/.test(entry.browser.file) || entry.browser.url !== `/fonts/${entry.browser.file}` || entry.browser.family !== entry.family || entry.browser.weight !== entry.weight || entry.browser.style !== (entry.style ?? "normal")))
                return Promise.reject(new OfflineFontError(displayMessage("fontReadiness.webNativeManifestMismatch", { p0: uiDisplayParameter(family) }), [family]));
        }
        if (normalizedFamily(family) === "new computer modern math" && !entries.some(entry => entry.role === "math" && entry.inspection?.hasMathTable === true))
            return Promise.reject(new OfflineFontError(uiMessage("fontReadiness.mathTableUnconfirmed"), [family]));
        const text = request.text ?? (normalizedFamily(family).includes("cjk") ? CHINESE_FONT_SAMPLE : normalizedFamily(family).includes("math") ? "∑∫√()abc0123" : "Weftext ABCxyz 0123");
        const font = `${style} ${weight} 16px ${cssFamily(family)}`;
        const key = `${font}\0${text}`;
        const active = this.inFlight.get(key);
        if (active)
            return active;
        const promise = bounded(this.host.fonts!.load(font, text), this.host.timeoutMs ?? 15000, displayMessage("fontReadiness.fontLoadTimeout", { p0: uiDisplayParameter(family) })).then(faces => {
            if (!faces.length || !faces.some(face => normalizedFamily(face.family) === normalizedFamily(family) && face.status === "loaded") || !this.host.fonts!.check(font, text))
                throw new OfflineFontError(displayMessage("fontReadiness.fontNotLoaded", { p0: uiDisplayParameter(family) }), [family]);
        }).catch(error => {
            throw error instanceof OfflineFontError ? new OfflineFontError(errorUiText(error), [family]) : new OfflineFontError(displayMessage("fontReadiness.fontLoadFailed", { p0: uiDisplayParameter(family), p1: uiDisplayParameter(errorUiText(error)) }), [family]);
        }).finally(() => { this.inFlight.delete(key); });
        this.inFlight.set(key, promise);
        return promise;
    }
}
let browserLoader: OfflineFontLoader | null = null;
function loader(): OfflineFontLoader {
    if (!browserLoader) {
        browserLoader = new OfflineFontLoader({
            fonts: typeof document === "undefined" ? undefined : document.fonts,
            readManifest: async () => {
                const response = await fetch("/api/fonts", { headers: { "X-Weftext-Token": sessionStorage.getItem("weftext-token") ?? "" } });
                if (!response.ok)
                    throw new UiError(displayMessage("fontReadiness.manifestHttpFailure", { p0: response.status }));
                return await response.json() as FontDeliveryManifest;
            },
        });
    }
    return browserLoader;
}
export async function ensureOfflineFonts(): Promise<void> { await loader().ensureDefaults(); }
export async function ensureRenderedFonts(requests: readonly RenderedFontRequest[]): Promise<void> { await loader().ensure(requests); }
