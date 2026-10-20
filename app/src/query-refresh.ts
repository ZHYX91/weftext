/** One active read and one latest waiting intent. No completed result is kept. */
export class QueryRefreshQueue<Input> {
  private active: { key: string; input: Input } | null = null;
  private pending: { key: string; input: Input } | null = null;
  private desired: string | null = null;
  private completion: Promise<void> | null = null;

  constructor(private readonly execute: (input: Input, current: () => boolean) => Promise<void>) {}

  invalidate(): Input | null { const pending = this.pending?.input ?? null; this.desired = null; this.pending = null; return pending; }

  request(key: string, input: Input): Promise<void> {
    this.desired = key;
    if (this.active?.key === key) this.pending = null;
    else if (this.pending?.key !== key) this.pending = { key, input: structuredClone(input) };
    this.completion ??= Promise.resolve().then(() => this.drain());
    return this.completion;
  }

  private async drain(): Promise<void> {
    let failure: unknown;
    try {
      while (this.pending) {
        const job = this.pending; this.pending = null; this.active = job;
        failure = undefined;
        try { await this.execute(job.input, () => this.desired === job.key); }
        catch (error) { if (this.desired === job.key) failure = error; }
        finally { this.active = null; }
      }
    } finally { this.completion = null; }
    if (failure !== undefined) throw failure;
  }
}

/** Object member order is not a different Query input; array order remains native. */
export function queryRefreshKey(input: unknown): string {
  return JSON.stringify(input, (_key, value: unknown) => value && typeof value === "object" && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value).sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)) : value);
}
