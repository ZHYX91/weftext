// Fixed CLI correspondence for owned acceptance harnesses. This does not turn
// an older receipt into a current-checkout build or transfer previous PASS results.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { readFile, stat, open } from 'node:fs/promises';
import path from 'node:path';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
export async function fileSha256(file) { const hash = createHash('sha256'); for await (const chunk of createReadStream(file))
    hash.update(chunk); return hash.digest('hex'); }
const runtime = name => name.startsWith('app/') || name.startsWith('assets/') || name.startsWith('crates/') || name.startsWith('resources/') || name.startsWith('platforms/') || name === 'docs/design/examples/schema.json' || ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'].includes(name);
export async function richArtifact(exe, repo, sourceRef = 'HEAD') {
    assert.ok(path.isAbsolute(exe));
    assert.match(path.basename(exe), /^weftext-cli\.exe$/i);
    const batch = path.dirname(exe), receiptBytes = await readFile(path.join(batch, 'build-input-receipt.json')), catalogBytes = await readFile(path.join(batch, 'artifact-catalog.json'));
    const receipt = JSON.parse(receiptBytes), catalog = JSON.parse(catalogBytes), executableSha256 = await fileSha256(exe), bytes = (await stat(exe)).size;
    assert.equal(receipt.schemaVersion, 1);
    assert.equal(catalog.schemaVersion, 1);
    assert.equal(receipt.batchId, path.basename(batch));
    assert.equal(receipt.target, 'x86_64-pc-windows-msvc');
    assert.deepEqual(receipt.features, []);
    assert.equal(receipt.catalog.sha256, sha(catalogBytes));
    assert.equal(receipt.catalog.bytes, catalogBytes.length);
    for (const entry of [receipt.roles.cli, catalog.roles.cli]) {
        assert.equal(entry.filename, path.basename(exe));
        assert.equal(entry.sha256, executableSha256);
        assert.equal(entry.bytes, bytes);
    }
    const file = await open(exe, 'r');
    try {
        const header = Buffer.alloc(512);
        await file.read(header, 0, header.length, 0);
        assert.equal(header.subarray(0, 2).toString(), 'MZ');
        const pe = header.readUInt32LE(0x3c);
        const native = Buffer.alloc(96);
        await file.read(native, 0, native.length, pe);
        assert.equal(native.subarray(0, 4).toString('hex'), '50450000');
        assert.equal(native.readUInt16LE(4), 0x8664);
        assert.equal(native.readUInt16LE(92), 3);
    }
    finally {
        await file.close();
    }
    const commit = execFileSync('git', ['rev-parse', `${sourceRef}^{commit}`], { cwd: repo, encoding: 'utf8', windowsHide: true }).trim();
    // Git blobs avoid checkout CRLF and concurrent changes in another worktree.
    const input = Buffer.from(receipt.source.map(entry => `${commit}:${entry.path}\n`).join(''));
    const output = execFileSync('git', ['cat-file', '--batch'], { cwd: repo, input, maxBuffer: 512 * 1024 * 1024, windowsHide: true });
    let at = 0;
    const differences = [], checkoutNewlines = [];
    for (const entry of receipt.source) {
        const end = output.indexOf(10, at);
        assert.ok(end >= at);
        const header = output.subarray(at, end).toString();
        at = end + 1;
        assert.ok(!header.endsWith(' missing'), `Missing source ${entry.path}`);
        const size = Number(header.split(' ').at(-1)), blob = output.subarray(at, at + size);
        at += size + 1;
        const actual = sha(blob);
        if (actual !== entry.sha256) {
            const text = blob.toString('utf8');
            const windows = Buffer.from(text.replace(/\r?\n/g, '\r\n'));
            if (!blob.includes(0) && Buffer.from(text).equals(blob) && windows.length === entry.bytes && sha(windows) === entry.sha256)
                checkoutNewlines.push({ path: entry.path, gitBlobSha256: actual, receiptSha256: entry.sha256, transformation: 'uniform CRLF checkout bytes' });
            else differences.push({ path: entry.path, receiptSha256: entry.sha256, sourceBlobSha256: actual });
        } else assert.equal(blob.length, entry.bytes, entry.path);
    }
    assert.equal(at, output.length);
    assert.deepEqual(differences.filter(entry => runtime(entry.path)), [], 'Selected CLI runtime source must equal the selected Git snapshot');
    const names = execFileSync('git', ['ls-tree', '-r', '--name-only', commit], { cwd: repo, encoding: 'utf8', windowsHide: true }).trim().split('\n'), known = new Set(receipt.source.map(entry => entry.path));
    assert.deepEqual(names.filter(name => runtime(name) && !known.has(name)), [], 'New runtime sources require a new artifact');
    return { exe, executableSha256, batch, batchId: receipt.batchId, receiptSha256: sha(receiptBytes), catalogSha256: sha(catalogBytes), sourceRef: commit, receiptSourceRoot: receipt.sourceRoot, sourceFilesVerified: receipt.source.length, checkoutNewlines, nonRuntimeDifferences: differences, frontend: receipt.frontend, scope: 'Physical CLI and original receipt/catalog; selected Git runtime blobs or their exact uniform CRLF checkout bytes match, newline mappings and nonruntime differences listed. Independent current harness, no old PASS transfer or current-checkout build claim.' };
}
export async function verifyEmbeddedFrontend(base, entries) {
    const result = [];
    for (const entry of entries) {
        assert.ok(!entry.path.includes('..') && !entry.path.startsWith('/'));
        const response = await fetch(base + '/' + entry.path, { signal: AbortSignal.timeout(30000) });
        assert.equal(response.status, 200, entry.path);
        const bytes = Buffer.from(await response.arrayBuffer());
        assert.equal(bytes.length, entry.bytes, entry.path);
        assert.equal(sha(bytes), entry.sha256, entry.path);
        result.push({ path: entry.path, sha256: entry.sha256, bytes: entry.bytes });
    }
    return result;
}
