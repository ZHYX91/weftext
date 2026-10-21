/** A skipped surface paint, not a cache or a request caused by focus itself. */
export class DeferredPaint {
  private pending?: { basis: readonly unknown[] };
  private queued?: { basis: readonly unknown[] };

  defer(basis: readonly unknown[]): void { this.pending = { basis }; }
  clear(): void { this.pending = undefined; this.queued = undefined; }

  flush(current: () => readonly unknown[] | undefined, retained: () => boolean, paint: () => void): void {
    const request = this.pending;
    if (!request || this.queued === request) return;
    this.queued = request;
    queueMicrotask(() => {
      if (this.queued === request) this.queued = undefined;
      if (this.pending !== request) return;
      const basis = current();
      if (!basis || basis.length !== request.basis.length || basis.some((value, index) => !Object.is(value, request.basis[index]))) { this.pending = undefined; return; }
      if (retained()) return;
      this.pending = undefined;
      paint();
    });
  }
}
