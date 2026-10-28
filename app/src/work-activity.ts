import type { WorkResult, WorkTask } from "./work-panel";

export type WorkActivityEvent =
    | { kind: "task"; task: WorkTask }
    | { kind: "result"; result: WorkResult }
    | { kind: "removeResult"; id: string };

/** Transient presentation of facts reported by the existing operation owners.
 * This observer never starts, retries, persists or settles an operation. */
export class WorkActivity {
    private readonly listeners = new Set<(event: WorkActivityEvent) => void>();
    private readonly tasks = new Map<string, WorkTask>();
    private readonly results = new Map<string, WorkResult>();
    constructor(private readonly retainedComplete = 128) {}
    subscribe(listener: (event: WorkActivityEvent) => void, options: { replay?: boolean } = {}): () => void {
        this.listeners.add(listener);
        if (options.replay !== false) {
            for (const task of this.tasks.values()) this.deliver(listener, { kind: "task", task });
            for (const result of this.results.values()) this.deliver(listener, { kind: "result", result });
        }
        return () => { this.listeners.delete(listener); };
    }
    task(task: WorkTask): void {
        if (!task.id) return;
        this.tasks.delete(task.id);
        this.tasks.set(task.id, task);
        // Never evict a pending original merely to fit a display history limit.
        const complete = [...this.tasks.values()].filter(value => value.status === "succeeded");
        for (const value of complete.slice(0, Math.max(0, complete.length - this.retainedComplete))) this.tasks.delete(value.id);
        this.emit({ kind: "task", task });
    }
    result(result: WorkResult): void {
        if (!result.id) return;
        this.results.delete(result.id);
        this.results.set(result.id, result);
        while (this.results.size > this.retainedComplete) {
            const id = this.results.keys().next().value;
            if (id === undefined) break;
            this.removeResult(id);
        }
        this.emit({ kind: "result", result });
    }
    removeResult(id: string): void {
        if (this.results.delete(id)) this.emit({ kind: "removeResult", id });
    }
    private deliver(listener: (event: WorkActivityEvent) => void, event: WorkActivityEvent): void {
        try { listener(event); }
        catch { /* A presentation failure must not change an author's operation outcome. */ }
    }
    private emit(event: WorkActivityEvent): void {
        for (const listener of this.listeners) this.deliver(listener, event);
    }
}

export const workActivity = new WorkActivity();
