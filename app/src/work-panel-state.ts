export type WorkPanelTab = "problems" | "tasks" | "results";
export type WorkTaskStatus = "running" | "waiting" | "failed" | "succeeded" | "cancelled";
export interface WorkPanelPreference {
    collapsed: boolean;
    height: number;
    selected: WorkPanelTab;
}
export interface WorkPanelStorage {
    getItem(key: string): string | null;
    setItem(key: string, value: string): void;
    removeItem(key: string): void;
}
export const workPanelTabs: readonly WorkPanelTab[] = ["problems", "tasks", "results"];
export const defaultWorkPanelPreference = (): WorkPanelPreference => ({ collapsed: true, height: 220, selected: "problems" });

/** Geometry only. No diagnostics, task payloads, results or author data are persisted. */
export function decodeWorkPanelPreference(value: unknown): WorkPanelPreference {
    const defaults = defaultWorkPanelPreference();
    if (!value || typeof value !== "object" || Array.isArray(value)) return defaults;
    const item = value as Record<string, unknown>;
    return {
        collapsed: typeof item.collapsed === "boolean" ? item.collapsed : defaults.collapsed,
        height: typeof item.height === "number" && Number.isFinite(item.height) && item.height >= 96 && item.height <= 720 ? Math.round(item.height) : defaults.height,
        selected: workPanelTabs.includes(item.selected as WorkPanelTab) ? item.selected as WorkPanelTab : defaults.selected,
    };
}

export function workPanelPreferenceKey(context: string): string {
    return `weftext-work-panel:v1:${encodeURIComponent(context)}`;
}

export class WorkPanelPreferenceStore {
    private readonly memory = new Map<string, WorkPanelPreference>();
    private reported = false;
    constructor(private readonly storage: WorkPanelStorage | null, private readonly unavailable?: () => void) {}
    private report(): void {
        if (this.reported) return;
        this.reported = true;
        this.unavailable?.();
    }
    read(context: string | null): WorkPanelPreference {
        if (!context) return defaultWorkPanelPreference();
        const key = workPanelPreferenceKey(context), cached = this.memory.get(key);
        if (cached) return { ...cached };
        let preference = defaultWorkPanelPreference();
        try {
            const raw = this.storage?.getItem(key);
            if (raw && raw.length <= 2048) {
                try { preference = decodeWorkPanelPreference(JSON.parse(raw)); }
                catch { /* Invalid display preferences reset without changing author state. */ }
            }
        } catch { this.report(); }
        this.memory.set(key, preference);
        return { ...preference };
    }
    write(context: string | null, preference: WorkPanelPreference): void {
        if (!context) return;
        const key = workPanelPreferenceKey(context), clean = decodeWorkPanelPreference(preference);
        this.memory.set(key, clean);
        try { this.storage?.setItem(key, JSON.stringify(clean)); }
        catch { this.report(); }
    }
    reset(context: string | null): void {
        if (!context) return;
        const key = workPanelPreferenceKey(context);
        this.memory.set(key, defaultWorkPanelPreference());
        try { this.storage?.removeItem(key); }
        catch { this.report(); }
    }
}

export function workPanelHeightBounds(viewportHeight: number): { min: number; max: number } {
    const height = Number.isFinite(viewportHeight) ? viewportHeight : 600;
    return { min: 96, max: Math.max(96, Math.min(720, Math.floor(height * .6))) };
}
export function clampWorkPanelHeight(height: number, viewportHeight: number): number {
    const bounds = workPanelHeightBounds(viewportHeight);
    return Math.round(Math.max(bounds.min, Math.min(bounds.max, height)));
}
export function keyboardWorkPanelHeight(height: number, key: string, shift: boolean, viewportHeight: number): number | null {
    const bounds = workPanelHeightBounds(viewportHeight);
    if (key === "Home") return bounds.min;
    if (key === "End") return bounds.max;
    if (key === "Enter") return clampWorkPanelHeight(defaultWorkPanelPreference().height, viewportHeight);
    if (key === "ArrowUp") return clampWorkPanelHeight(height + (shift ? 48 : 12), viewportHeight);
    if (key === "ArrowDown") return clampWorkPanelHeight(height - (shift ? 48 : 12), viewportHeight);
    return null;
}
export function workTaskNeedsAttention(status: WorkTaskStatus): boolean { return status !== "succeeded" && status !== "cancelled"; }
export function workTaskProgress(value: unknown): number | undefined {
    return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 100 ? Math.round(value) : undefined;
}
