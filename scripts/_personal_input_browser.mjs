// Read the actual committed proposal stores together; never create or modify a
// database while inspecting privacy or recovery evidence in an owned browser.
export async function personalInputSnapshot(page) {
  return page.evaluate(async () => {
    const name = 'weftext-personal-input-v3';
    if (!(await indexedDB.databases()).some(database => database.name === name)) {
      throw Error('Expected personal-input database has not been created');
    }
    const database = await new Promise((resolve, reject) => {
      const request = indexedDB.open(name);
      request.onupgradeneeded = () => { request.transaction.abort(); reject(Error('Inspection must not create a database')); };
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(Error('Personal-input inspection was blocked'));
      request.onsuccess = () => resolve(request.result);
    });
    try {
      return await new Promise((resolve, reject) => {
        const transaction = database.transaction(['audiences', 'texts'], 'readonly');
        const audiences = transaction.objectStore('audiences').getAll();
        const texts = transaction.objectStore('texts').getAll();
        transaction.oncomplete = () => resolve({ audiences: audiences.result, texts: texts.result });
        transaction.onabort = transaction.onerror = () => reject(transaction.error ?? Error('Personal-input snapshot failed'));
      });
    } finally { database.close(); }
  });
}

// Abort the real target readwrite transaction, leaving readonly inspection and
// unrelated browser stores intact. This is a controlled failure, not OS quota.
export async function abortPersonalInputWrites(page) {
  await page.evaluate(() => {
    const original = IDBDatabase.prototype.transaction;
    window.personalInputWriteAborts = 0;
    IDBDatabase.prototype.transaction = function (...args) {
      const transaction = original.apply(this, args);
      if (this.name === 'weftext-personal-input-v3' && args[1] === 'readwrite') {
        queueMicrotask(() => { transaction.abort(); window.personalInputWriteAborts++; });
      }
      return transaction;
    };
  });
}
