/** One transactional browser proposal store. No token, protected source, author
 * effect or Core close proof belongs to this backend. UTF-16 strings are exact. */
export interface PersonalInputBundle { commitId: number; metadata: string; texts: ReadonlyMap<string, string> }
export interface PersonalInputBackend {
  read(audience: string): Promise<PersonalInputBundle | undefined>;
  compareAndWrite(audience: string, expectedCommit: number, metadata: string, texts: ReadonlyMap<string, string>): Promise<number>;
}
interface Row { audience: string; commitId: number; metadata: string; texts: { id: string; units: number }[] }
interface TextRow { audience: string; id: string; text: string }
const MAX_METADATA_BYTES = 16 * 1024 * 1024;
const MAX_DOMAIN_BYTES = 64 * 1024 * 1024;
const MAX_TEXTS = 4096;
const validAudience = (value: string) => !!value && value.length <= 4096;
const validId = (value: string) => /^[a-f0-9]{32}$/.test(value);
const integer = (value: number) => Number.isSafeInteger(value) && value >= 0;
export class PersonalInputStoreConflict extends Error { constructor() { super("另一页面已更新本人恢复记录；原件和当前输入保留，未覆盖另一 producer。"); } }
function budget(metadata: string, texts: ReadonlyMap<string, string>): void {
  let bytes = metadata.length * 2;
  if (bytes > MAX_METADATA_BYTES || texts.size > MAX_TEXTS) throw new Error("本人输入恢复记录超过有界存储预算，原件保留。");
  for (const [id, text] of texts) {
    if (!validId(id) || typeof text !== "string") throw new Error("本人输入文本引用无效，原件保留。");
    bytes += text.length * 2;
    if (bytes > MAX_DOMAIN_BYTES) throw new Error("本人输入恢复域超过有界存储预算，原件保留。");
  }
}
function row(value: unknown, audience: string): Row {
  const actual = value as Row;
  if (!actual || actual.audience !== audience || !integer(actual.commitId) || actual.commitId < 1 || typeof actual.metadata !== "string" || actual.metadata.length * 2 > MAX_METADATA_BYTES || !Array.isArray(actual.texts) || actual.texts.length > MAX_TEXTS) throw new Error("本人持久记录无法完整读取；未采用或覆盖原件。");
  const ids = new Set<string>(); let bytes = actual.metadata.length * 2;
  for (const text of actual.texts) {
    if (!text || !validId(text.id) || ids.has(text.id) || !integer(text.units)) throw new Error("本人文本清单无效，原件保留。");
    ids.add(text.id); bytes += text.units * 2;
    if (bytes > MAX_DOMAIN_BYTES) throw new Error("本人持久记录超出预算，原件保留。");
  }
  return actual;
}

/** Metadata plus immutable text chunks share one IndexedDB readwrite CAS
 * transaction. Failed/quota/aborted transactions leave the old pointer intact. */
export class IndexedDBPersonalInputBackend implements PersonalInputBackend {
  private database?: Promise<IDBDatabase>;
  constructor(private readonly factory: IDBFactory = indexedDB, private readonly name = "weftext-personal-input-v3") {}
  private open(): Promise<IDBDatabase> {
    return this.database ??= new Promise((resolve, reject) => {
      const request = this.factory.open(this.name, 1);
      let refused = false;
      request.onupgradeneeded = () => {
        const database = request.result;
        database.createObjectStore("audiences", { keyPath: "audience" });
        database.createObjectStore("texts", { keyPath: ["audience", "id"] });
      };
      request.onerror = () => { this.database = undefined; reject(request.error ?? new Error("本人输入数据库暂不可用。")); };
      request.onblocked = () => { refused = true; reject(new Error("本人输入数据库升级被旧页面占用，原件保留。")); };
      request.onsuccess = () => { const database = request.result; if (refused) { database.close(); this.database = undefined; return; } database.onversionchange = () => { database.close(); this.database = undefined; }; resolve(database); };
    });
  }
  async read(audience: string): Promise<PersonalInputBundle | undefined> {
    if (!validAudience(audience)) throw new Error("缺少准确本人恢复域。");
    const database = await this.open();
    return await new Promise((resolve, reject) => {
      const transaction = database.transaction(["audiences", "texts"], "readonly");
      let result: PersonalInputBundle | undefined, failure: unknown;
      const metadata = transaction.objectStore("audiences").get(audience);
      const fail = (error: unknown) => { failure = error; transaction.abort(); };
      metadata.onsuccess = () => {
        if (metadata.result === undefined) return;
        let selected: Row; try { selected = row(metadata.result, audience); } catch (error) { fail(error); return; }
        const texts = new Map<string, string>(); result = { commitId: selected.commitId, metadata: selected.metadata, texts };
        for (const entry of selected.texts) {
          const request = transaction.objectStore("texts").get([audience, entry.id]);
          request.onsuccess = () => {
            const actual = request.result as TextRow | undefined;
            if (!actual || actual.audience !== audience || actual.id !== entry.id || typeof actual.text !== "string" || actual.text.length !== entry.units) { fail(new Error("本人文本快照缺失或已变化，原件保留。")); return; }
            texts.set(entry.id, actual.text);
          };
        }
      };
      transaction.oncomplete = () => resolve(result);
      transaction.onabort = transaction.onerror = () => reject(failure ?? transaction.error ?? new Error("本人输入读取未完成。"));
    });
  }
  async compareAndWrite(audience: string, expectedCommit: number, metadata: string, texts: ReadonlyMap<string, string>): Promise<number> {
    if (!validAudience(audience) || !integer(expectedCommit) || expectedCommit === Number.MAX_SAFE_INTEGER) throw new Error("本人持久修订无效，原件保留。");
    budget(metadata, texts);
    // Freeze the exact caller cut before opening an asynchronous transaction.
    const textCut = new Map(texts), database = await this.open();
    return await new Promise((resolve, reject) => {
      const transaction = database.transaction(["audiences", "texts"], "readwrite", { durability: "strict" });
      const metadataStore = transaction.objectStore("audiences"), textStore = transaction.objectStore("texts");
      let failure: unknown;
      const fail = (error: unknown) => { failure = error; transaction.abort(); };
      const original = metadataStore.get(audience);
      original.onsuccess = () => {
        let previous: Row | undefined;
        try { previous = original.result === undefined ? undefined : row(original.result, audience); } catch (error) { fail(error); return; }
        if ((previous?.commitId ?? 0) !== expectedCommit) { fail(new PersonalInputStoreConflict()); return; }
        for (const [id, text] of textCut) {
          const existing = textStore.get([audience, id]);
          existing.onsuccess = () => {
            if (existing.result !== undefined) {
              const old = existing.result as TextRow;
              if (old.audience !== audience || old.id !== id || old.text !== text) fail(new Error("不可变本人文本快照身份冲突，原件保留。"));
            } else textStore.add({ audience, id, text } satisfies TextRow);
          };
        }
        // Only chunks owned by the previous committed manifest are retired.
        for (const old of previous?.texts ?? []) if (!textCut.has(old.id)) textStore.delete([audience, old.id]);
        metadataStore.put({ audience, commitId: expectedCommit + 1, metadata, texts: [...textCut].map(([id, text]) => ({ id, units: text.length })) } satisfies Row);
      };
      transaction.oncomplete = () => resolve(expectedCommit + 1);
      transaction.onabort = transaction.onerror = () => reject(failure ?? transaction.error ?? new Error("本人输入写入未完成；最新输入仍在内存，旧持久记录保留。"));
    });
  }
}
