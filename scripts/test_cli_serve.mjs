// Pure protocol/argument regressions. No product, browser or fixture process runs.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EventEmitter } from 'node:events';
import { readFileSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { PassThrough } from 'node:stream';
import { fileURLToPath } from 'node:url';
import { parseServeJson, stringifyServeJson, serveArguments, spawnServe, serveReady, stopServe, validateServeReady, validateOwnerStop } from './_cli_serve.mjs';

const owner = { pid: 412, created: 134055000000000001n, incarnation: 'original incarnation', profileIdentity: 'original physical profile', role: 'local' };
const token = 'a'.repeat(64);
const ready = (identity = owner, url = 'http://127.0.0.1:41234/#token=' + token) => stringifyServeJson({ schemaVersion: 1, kind: 'ready', url, ownerIdentity: identity });
const stopped = (identity = owner) => stringifyServeJson({ schemaVersion: 1, call: 'ownerStop', result: { owner: identity, exitCode: 0 } });

test('current command preserves explicit initial-document/port and refuses legacy or unowned inputs', async () => {
  assert.deepEqual(serveArguments(['--data-dir', 'D:/TEST']), ['serve', '--data-dir', 'D:/TEST', '--port', '0']);
  assert.deepEqual(serveArguments(['--port=0', '--open=D:/TEST/a.typ', '--root=D:/TEST']), ['serve', '--port=0', '--open=D:/TEST/a.typ', '--root=D:/TEST']);
  assert.deepEqual(serveArguments(['--data-dir', 'serve']), ['serve', '--data-dir', 'serve', '--port', '0']);
  assert.deepEqual(serveArguments(['--server-config=D:/TEST/operator.json']), ['serve', '--server-config=D:/TEST/operator.json']);
  assert.deepEqual(serveArguments(['--server-config', 'D:/TEST/operator.json', '--port', '0']), ['serve', '--server-config', 'D:/TEST/operator.json', '--port', '0']);
  for (const args of [['--no-browser'], ['serve'], [false], ['--root=D:/TEST']]) assert.throws(() => serveArguments(args));
  assert.throws(() => spawnServe('weftext-cli.exe', [], {}));
  assert.throws(() => serveReady({}));
  await assert.rejects(stopServe({}), /unowned/);
});

test('u64 source digits survive JSON parse/write above Number precision', () => {
  const parsed = parseServeJson('{"created":18446744073709551615}');
  assert.equal(parsed.created, 18446744073709551615n);
  assert.equal(stringifyServeJson(parsed), '{"created":18446744073709551615}');
  assert.equal(validateServeReady(ready(), 412, 'local').ownerIdentity.created, owner.created);
  for (const value of ['-1', '-0', '1.0', '1e2', '18446744073709551616', '"1"']) {
    assert.throws(() => parseServeJson('{"created":' + value + '}'));
  }
  assert.equal(parseServeJson('0' + ' '.repeat(16 * 1024 - 1)), 0);
  assert.throws(() => parseServeJson('0' + ' '.repeat(16 * 1024)), /16 KiB/);
  assert.throws(() => parseServeJson(JSON.stringify('汉'.repeat(6000))), /16 KiB/);
});

test('Ready binds actual PID/role and strict schema while reading the actual URL token', () => {
  const actual = validateServeReady(ready(), 412, 'local');
  assert.equal(actual.base, 'http://127.0.0.1:41234');
  assert.equal(actual.token, token);
  assert.equal(actual.ownerIdentity.incarnation, owner.incarnation);
  assert.throws(() => validateServeReady(ready(), 413, 'local'));
  assert.throws(() => validateServeReady(ready(), 412, 'hosted'));
  const extra = parseServeJson(ready()); extra.additional = true;
  assert.throws(() => validateServeReady(stringifyServeJson(extra), 412, 'local'));
  for (const url of [`http://127.0.0.1:0/#token=${token}`, `http://127.0.0.1:41234/?token=${token}`, `http://127.0.0.1:41234/#token=${token}&token=${token}`, `https://unrelated.invalid/#token=${token}`, `http://@127.0.0.1:41234/#token=${token}`, `http://secret@127.0.0.1:41234/#token=${token}`, `http://remote.invalid:41234/#token=${token}`, `http://localhost:41234/#token=${token}`]) assert.throws(() => validateServeReady(ready(owner, url), 412, 'local'));
  for (const invalid of ['a'.repeat(63), 'a'.repeat(65), 'A'.repeat(64), 'z'.repeat(64), 'TEST@secret']) assert.throws(() => validateServeReady(ready(owner, 'http://127.0.0.1:41234/#token=' + invalid), 412, 'local'), /browser token/);
  for (const field of ['incarnation', 'profileIdentity']) {
    assert.equal(validateServeReady(ready({ ...owner, [field]: 'x'.repeat(4096) }), 412, 'local').ownerIdentity[field].length, 4096);
    assert.throws(() => validateServeReady(ready({ ...owner, [field]: 'x'.repeat(4097) }), 412, 'local'), /Invalid owner/);
  }
});

test('Hosted Ready exposes an origin and no operator secret', () => {
  const hosted = { ...owner, role: 'hosted' };
  assert.equal(validateServeReady(ready(hosted, 'http://127.0.0.1:41234/'), 412, 'hosted').token, undefined);
  assert.equal(validateServeReady(ready(hosted, 'http://operator.invalid:41234/'), 412, 'hosted').base, 'http://operator.invalid:41234');
  assert.throws(() => validateServeReady(ready(hosted, 'https://operator.invalid:41234/'), 412, 'hosted'));
  assert.throws(() => validateServeReady(ready(hosted), 412, 'hosted'));
});

test('successful stop needs the exact original owner, including low u64 bits', () => {
  assert.equal(validateOwnerStop(stopped(), owner).result.exitCode, 0);
  for (const identity of [{ ...owner, created: owner.created - 1n }, { ...owner, pid: 413 }, { ...owner, incarnation: 'other' }, { ...owner, profileIdentity: 'other' }, { ...owner, role: 'hosted' }]) {
    assert.throws(() => validateOwnerStop(stopped(identity), owner));
  }
  for (const edit of [value => value.call = 'other', value => value.result.exitCode = 6, value => value.result.additional = true]) {
    const value = parseServeJson(stopped()); edit(value); assert.throws(() => validateOwnerStop(stringifyServeJson(value), owner));
  }
  for (const field of ['incarnation', 'profileIdentity']) assert.throws(() => validateOwnerStop(stopped({ ...owner, [field]: 'x'.repeat(4097) }), owner), /Invalid owner/);
});

test('mocked lifecycle stops once, preserves exact u64, and never retries/kills unknown or missing Ready', async () => {
  // EventEmitter fixtures simulate only the process boundary. No executable is
  // spawned; retained TEST evidence lives in the product's ignored target/.
  const testRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../target/weftext-checks');
  await mkdir(testRoot, { recursive: true });
  const directory = await mkdtemp(path.join(testRoot, 'cli-serve-mock-'));
  for (const scenario of ['success', 'unknown', 'missing-ready']) {
    let kills = 0;
    class MockChild extends EventEmitter {
      constructor(pid) { super(); this.pid = pid; this.stdout = new PassThrough(); this.stderr = new PassThrough(); this.exitCode = null; this.signalCode = null; }
      kill() { kills++; throw new Error('A mocked product process must never be killed'); }
      finish(code) { this.exitCode = code; this.stdout.end(); this.stderr.end(); this.emit('close', code, null); }
    }
    const actualOwner = new MockChild(owner.pid), commands = [];
    const spawnMock = (executable, args) => {
      commands.push({ executable, args });
      if (args[0] === 'serve') return actualOwner;
      assert.equal(args[0], 'owner-stop');
      const identityFile = args[args.indexOf('--owner-identity') + 1];
      const identityText = readFileSync(identityFile, 'utf8');
      assert.match(identityText, /"created": 134055000000000001/);
      assert.equal(parseServeJson(identityText).created, owner.created);
      const control = new MockChild(413);
      queueMicrotask(() => {
        if (scenario === 'unknown') control.stderr.write('Outcome unknown; original request may have arrived\n');
        else control.stdout.write(stopped() + '\n');
        control.finish(scenario === 'unknown' ? 6 : 0);
        if (scenario === 'success') actualOwner.finish(0);
      });
      return control;
    };
    const data = path.join(directory, scenario, 'state');
    const child = spawnServe(process.execPath, ['--data-dir', data], { stdio: ['ignore', 'pipe', 'pipe'] }, spawnMock);
    if (scenario !== 'missing-ready') child.stdout.write(ready() + '\n');
    const first = stopServe(child), second = stopServe(child);
    assert.equal(first, second);
    if (scenario === 'success') await first;
    else await assert.rejects(first, scenario === 'unknown' ? /unknown \(6\)/ : /No qualified Ready identity/);
    assert.equal(commands.length, scenario === 'missing-ready' ? 1 : 2);
    assert.equal(kills, 0);
    const folders = await readdir(path.dirname(data));
    const evidence = path.join(path.dirname(data), folders.find(name => name.startsWith('owner-control-')));
    await writeFile(path.join(evidence, 'MOCK-FIXTURE.json'), JSON.stringify({
      scope: 'Unit test using EventEmitter/PassThrough; not a Weftext process or product acceptance',
      scenario, syntheticPids: true, actualProductProcessesStarted: 0,
      actualFixtureProcessesStarted: 0, killCalls: kills, mockSpawnCalls: commands.length,
    }, null, 2), { flag: 'wx' });
    const receipt = parseServeJson(await readFile(path.join(evidence, 'lifecycle.json'), 'utf8'));
    assert.equal(receipt.status.startsWith('PASS'), scenario === 'success');
    assert.equal(receipt.startupFaultStopAttempted, false);
    if (scenario !== 'success') actualOwner.finish(0); // Dispose a mock stream, not an OS process.
  }
  console.log('Owned mock lifecycle evidence: ' + directory);
});
