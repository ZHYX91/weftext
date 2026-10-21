/** Track input ownership as edits occur; an old substring between two edits is never recovered. */
export class DocumentNotesInputTracker {
  private value: string;
  private owned: boolean[];
  private detached: string[] = [];
  constructor(value: string) { this.value = value; this.owned = Array(value.length).fill(false); }
  copy(): DocumentNotesInputTracker { const copy = new DocumentNotesInputTracker(this.value); copy.owned = [...this.owned]; copy.detached = [...this.detached]; return copy; }
  matches(value: string): boolean { return this.value === value; }
  /** DOM insertions require the event's payload, including every IME update.
   * Prefix/suffix alone cannot distinguish an unobserved programmatic reset. */
  observe(value: string, edit: { from: number; to: number; inputType: string } | undefined, data: string | null | undefined): void {
    if (edit?.inputType.startsWith("history") || edit && !edit.inputType.startsWith("insert")) { this.input(value, edit); return; }
    const fresh = edit?.inputType !== "insertFromDrop" && typeof data === "string";
    if (fresh && edit && Number.isSafeInteger(edit.from) && Number.isSafeInteger(edit.to) && edit.from >= 0 && edit.to >= edit.from && edit.to <= this.value.length && value === this.value.slice(0, edit.from) + data + this.value.slice(edit.to)) { this.input(value, edit); return; }
    // Only the observed payload is independent author text. The resulting DOM
    // value is opaque when the original edit cannot be proved.
    this.input(value);
    if (fresh && data) this.detached.push(data);
  }
  input(value: string, edit?: { from: number; to: number; inputType: string }): void {
    if (edit?.inputType.startsWith("history")) {
      // Native textarea history exposes no history token. Equal text can have
      // different origins, so it cannot restore ownership by value lookup.
      const known = this.newInput(); if (known) this.detached.push(known);
      this.owned = Array(value.length).fill(false); this.value = value; return;
    }
    if (value === this.value && !edit) return;
    const exact = edit && Number.isSafeInteger(edit.from) && Number.isSafeInteger(edit.to) && edit.from >= 0 && edit.to >= edit.from && edit.to <= this.value.length && value.startsWith(this.value.slice(0, edit.from)) && value.endsWith(this.value.slice(edit.to)) && value.length >= edit.from + this.value.length - edit.to;
    if (exact) {
      const inserted = value.length - this.value.length + edit.to - edit.from;
      // A drag may move source text. It is not independent new author input.
      const fresh = edit.inputType.startsWith("insert") && edit.inputType !== "insertFromDrop";
      this.owned = [...this.owned.slice(0, edit.from), ...Array(inserted).fill(fresh), ...this.owned.slice(edit.to)];
    } else {
      // Missing/ambiguous browser edit evidence cannot grant old text recovery.
      const known = this.newInput(); if (known) this.detached.push(known);
      this.owned = Array(value.length).fill(false);
    }
    this.value = value;
  }
  newInput(): string {
    let result = "", separated = false;
    for (let at = 0; at < this.value.length; at++) { if (this.owned[at]) { if (separated && result) result += "\n"; result += this.value[at]; separated = false; } else separated = !!result; }
    return result;
  }
  recoveryInput(): string { return [...this.detached, this.newInput()].filter(Boolean).join("\n"); }
}
