import { UiError, message } from "./ui-locale";

/** One lifetime covers the complete asynchronous caller, including result
 * delivery after transport resolution and time spent waiting for a dialog. */
export class LibraryCloseActivity {
  private pending = 0;
  private revision = 0;
  private frozen = false;

  begin(): () => void {
    this.assertAvailable();
    this.pending++;
    this.revision++;
    let live = true;
    return () => {
      if (!live) return;
      live = false;
      this.pending--;
      this.revision++;
    };
  }

  async run<T>(operation: () => Promise<T>): Promise<T> {
    const finish = this.begin();
    try { return await operation(); }
    finally { finish(); }
  }

  assertAvailable(): void {
    if (this.frozen) throw new UiError(message("shell.closePreparationFrozen"));
  }

  checkpoint(): number {
    if (this.frozen || this.pending !== 0) throw new UiError(message("shell.closeAuthorInputPending"));
    return this.revision;
  }

  freeze(checkpoint: number): boolean {
    if (this.frozen || this.pending !== 0 || checkpoint !== this.revision) return false;
    this.frozen = true;
    return true;
  }

  thaw(): void { this.frozen = false; }
}
