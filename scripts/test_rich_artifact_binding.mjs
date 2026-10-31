// Synthetic artifact/receipt and isolated Git fixture; never executes the fake PE.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { richArtifact } from './_rich_test_artifact.mjs';

const root = path.resolve(import.meta.dirname, '..'), out = process.argv[2];
assert.equal(process.argv.length, 3);
assert.ok(out && path.isAbsolute(out));
const relative = path.relative(root, out);
assert.ok(path.isAbsolute(relative) || relative.startsWith('..' + path.sep));
await mkdir(out);
const repo = path.join(out, 'repo'), batch = path.join(out, 'fixture-batch');
await mkdir(repo); await mkdir(batch);
const git = (...args) => execFileSync('git', args, { cwd: repo, encoding: 'utf8', windowsHide: true }).trim();
git('init', '--quiet'); git('config', 'user.name', 'Artifact fixture'); git('config', 'user.email', 'fixture@example.invalid');
git('config', 'core.autocrlf', 'false');
const files = ['crates/core.rs', 'docs/design/examples/schema.authoring.json', 'packaging/windows/host-contract.json', 'scripts/recognition/worker.py', 'brand/app-icon.svg'];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
for (const name of files) { await mkdir(path.dirname(path.join(repo, name)), { recursive: true }); await writeFile(path.join(repo, name), 'original\n'); }
git('add', '.'); git('commit', '--quiet', '-m', 'original fixture');
const original = git('rev-parse', 'HEAD'), pe = Buffer.alloc(608);
pe.write('MZ'); pe.writeUInt32LE(512, 0x3c); pe.write('PE', 512); pe.writeUInt16LE(0x8664, 516); pe.writeUInt16LE(3, 604);
const exe = path.join(batch, 'weftext-cli.exe'); await writeFile(exe, pe);
const role = { filename: 'weftext-cli.exe', bytes: pe.length, sha256: sha(pe) };
const catalog = Buffer.from(JSON.stringify({ schemaVersion: 1, roles: { cli: role } }));
await writeFile(path.join(batch, 'artifact-catalog.json'), catalog);
await writeFile(path.join(batch, 'build-input-receipt.json'), JSON.stringify({ schemaVersion: 1, batchId: 'fixture-batch', target: 'x86_64-pc-windows-msvc', features: [], roles: { cli: role }, catalog: { bytes: catalog.length, sha256: sha(catalog) }, sourceRoot: repo, source: files.map(name => ({ path: name, bytes: 9, sha256: sha('original\n') })), frontend: [] }));
await richArtifact(exe, repo, original);
const rejected = [];
for (const name of files) {
    await writeFile(path.join(repo, name), 'changed\n'); git('add', name); git('commit', '--quiet', '-m', 'changed fixture');
    await assert.rejects(richArtifact(exe, repo, 'HEAD'), /Every packaged source/);
    rejected.push(name);
    // Restore by a normal commit; do not rewrite history or delete fixture files.
    await writeFile(path.join(repo, name), 'original\n'); git('add', name); git('commit', '--quiet', '-m', 'restored fixture');
}
await writeFile(path.join(repo, 'new-runtime.json'), '{}'); git('add', '.'); git('commit', '--quiet', '-m', 'additional input');
await assert.rejects(richArtifact(exe, repo, 'HEAD'), /source inventory/);
for (const [target, expected] of [['C:\\review\\out', true], ['D:\\review\\out', true], ['C:\\src\\weftext\\out', false], ['C:\\src\\weftext', false]]) {
    const rel = path.win32.relative('C:\\src\\weftext', target);
    assert.equal(path.win32.isAbsolute(rel) || rel.startsWith('..' + path.win32.sep), expected);
}
await writeFile(path.join(out, 'result.json'), JSON.stringify({ completed: true, originalCommit: original, matchingAccepted: true, changedInputsRejected: rejected, newInputRejected: true, windowsPathCases: 4, scope: 'Synthetic receipt and Git validation only; fake PE never executed' }, null, 2));
console.log('PASS: exact match, five isolated embedded-input mismatches, added input, four Windows path cases');
