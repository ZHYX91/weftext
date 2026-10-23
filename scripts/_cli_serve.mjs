// Browser/HTTP harnesses receive the actual console role from the product build.
// This helper only starts that role; it never finds or builds an executable.
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const states = new WeakMap();
const limit = 4 * 1024 * 1024;
const MAX_MESSAGE = 16 * 1024;
let runtimeChecked = false;

function closed(value, keys, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value) || Object.keys(value).length !== keys.length || Object.keys(value).some(key => !keys.includes(key))) {
    throw new Error(`${label} has missing or unknown fields`);
  }
}

export function requireServeRuntime() {
  if (runtimeChecked) return;
  let source;
  JSON.parse('18446744073709551615', (_key, value, context) => { source = context?.source; return value; });
  if (Number(process.versions.node.split('.')[0]) < 24 || typeof JSON.rawJSON !== 'function' || source !== '18446744073709551615') {
    throw Object.assign(new Error('NONRUN: browser harness requires Node 24+ with JSON.parse context.source and JSON.rawJSON'), { code: 'NONRUN' });
  }
  runtimeChecked = true;
}

function u64(source) {
  if (typeof source !== 'string' || source.length > 20 || !/^(0|[1-9][0-9]*)$/.test(source) || BigInt(source) > 18446744073709551615n) {
    throw new Error('Owner created must be a canonical nonnegative decimal u64');
  }
  return BigInt(source);
}

export function parseServeJson(text) {
  requireServeRuntime();
  if (typeof text !== 'string' || Buffer.byteLength(text, 'utf8') > MAX_MESSAGE) throw new Error('Protocol message exceeds the 16 KiB budget');
  return JSON.parse(text, (key, value, context) => {
    if (key === 'created') {
      if (typeof value !== 'number') throw new Error('Owner created must be a JSON u64 number');
      return u64(context?.source);
    }
    if (typeof value === 'number' && (!Number.isSafeInteger(value) || !/^(0|[1-9][0-9]*)$/.test(context?.source))) {
      throw new Error('Unexpected noncanonical or unsafe protocol number');
    }
    return value;
  });
}

export function stringifyServeJson(value, space) {
  requireServeRuntime();
  return JSON.stringify(value, (_key, item) => typeof item === 'bigint' ? JSON.rawJSON(u64(item.toString()).toString()) : item, space);
}

function ownerIdentity(value) {
  closed(value, ['pid', 'created', 'incarnation', 'profileIdentity', 'role'], 'Owner identity');
  if (!Number.isSafeInteger(value.pid) || value.pid <= 0 || value.pid > 4294967295 || typeof value.created !== 'bigint') throw new Error('Invalid owner process identity');
  for (const field of ['incarnation', 'profileIdentity']) {
    if (typeof value[field] !== 'string' || !value[field] || value[field].length > 4096) throw new Error(`Invalid owner ${field}`);
  }
  if (!['local', 'hosted'].includes(value.role)) throw new Error('Invalid owner role');
  return value;
}

export function validateServeReady(text, pid, role) {
  const value = parseServeJson(text);
  closed(value, ['schemaVersion', 'kind', 'url', 'ownerIdentity'], 'Ready record');
  if (value.schemaVersion !== 1 || value.kind !== 'ready' || typeof value.url !== 'string') throw new Error('Invalid Ready record');
  const owner = ownerIdentity(value.ownerIdentity);
  if (owner.pid !== pid || owner.role !== role) throw new Error('Ready owner does not identify the actual spawned CLI role');
  const address = new URL(value.url);
  const authorityStart = value.url.indexOf('://');
  const authority = authorityStart < 0 ? '' : value.url.slice(authorityStart + 3).split(/[/?#]/, 1)[0];
  if (address.protocol !== 'http:' || authority.includes('@') || address.username || address.password || address.pathname !== '/' || address.search || address.port === '0') {
    throw new Error('Ready URL is not the owned service root without userinfo');
  }
  let token;
  if (role === 'local') {
    if (!value.url.startsWith('http://') || address.hostname !== '127.0.0.1' || authority !== '127.0.0.1:' + (address.port || '80')) {
      throw new Error('Local Ready URL differs from LocalService browser_url');
    }
    const fragment = new URLSearchParams(address.hash.slice(1));
    if ([...fragment.keys()].join(',') !== 'token' || !/^[0-9a-f]{64}$/.test(fragment.get('token') ?? '')) throw new Error('Local Ready URL lacks its exact browser token');
    token = fragment.get('token');
  } else if (address.hash) {
    throw new Error('Hosted Ready URL must not expose an operator secret');
  }
  return Object.freeze({ url: value.url, base: address.origin, token, ownerIdentity: Object.freeze(owner) });
}

export function validateOwnerStop(text, expected) {
  const value = parseServeJson(text);
  closed(value, ['schemaVersion', 'call', 'result'], 'OwnerStop response');
  closed(value.result, ['owner', 'exitCode'], 'OwnerStop result');
  if (value.schemaVersion !== 1 || value.call !== 'ownerStop' || value.result.exitCode !== 0) throw new Error('OwnerStop did not acknowledge a successful drain');
  const observed = ownerIdentity(value.result.owner);
  if (['pid', 'created', 'incarnation', 'profileIdentity', 'role'].some(field => observed[field] !== expected[field])) throw new Error('OwnerStop receipt differs from the exact Ready owner');
  return value;
}

function option(args, name) {
  const index = args.findIndex(argument => argument === name || argument.startsWith(name + '='));
  return index < 0 ? undefined : args[index] === name ? args[index + 1] : args[index].slice(name.length + 1);
}

export function serveArguments(args) {
  if (!Array.isArray(args) || args.some(argument => typeof argument !== 'string')) {
    throw new TypeError('CLI serve arguments must be an array of strings');
  }
  if (args.includes('--no-browser') || args[0] === 'serve') {
    throw new Error('Supply current serve options without a command or legacy GUI flag');
  }
  const hasOption = name => args.some(argument => argument === name || argument.startsWith(name + '='));
  if (hasOption('--root') && !hasOption('--open')) {
    throw new Error('--root requires an explicit --open initial document');
  }
  return ['serve', ...args, ...(hasOption('--port') || hasOption('--server-config') ? [] : ['--port', '0'])];
}

export function spawnServe(executable, args, options, spawnChild = spawn) {
  requireServeRuntime();
  if (typeof executable !== 'string' || !path.isAbsolute(executable)) {
    throw new TypeError('The actual CLI executable must be an absolute path');
  }
  const command = serveArguments(args), data = option(args, '--data-dir');
  if (!data || !path.isAbsolute(data)) throw new Error('Serve requires an explicit absolute owned TEST data directory');
  if (options?.stdio && (!Array.isArray(options.stdio) || options.stdio[1] !== 'pipe' || options.stdio[2] !== 'pipe')) {
    throw new Error('Serve requires observed stdout and stderr pipes');
  }
  const child = spawnChild(executable, command, { ...options, stdio: options?.stdio ?? ['ignore', 'pipe', 'pipe'] });
  const state = {
    executable, args: command, data, env: options?.env, spawnChild,
    stdout: [], stderr: [], stdoutBytes: 0, stderrBytes: 0,
    pending: Buffer.alloc(0),
    role: option(args, '--server-config') === undefined ? 'local' : 'hosted',
  };
  state.evidence = path.join(path.dirname(data), `owner-control-${child.pid ?? 'unspawned'}-${randomUUID()}`);
  states.set(child, state);
  child.on('error', error => { state.error = error; });
  state.exited = new Promise(resolve => child.once('close', (code, signal) => { state.exit = { code, signal }; resolve(state.exit); }));
  child.stderr.on('data', bytes => {
    state.stderrBytes += bytes.length;
    if (state.stderrBytes <= limit) state.stderr.push(Buffer.from(bytes));
    else state.error ??= new Error('Owned service stderr exceeds the evidence budget');
  });
  child.stdout.on('data', bytes => {
    state.stdoutBytes += bytes.length;
    if (state.stdoutBytes > limit) { state.error ??= new Error('Owned service stdout exceeds the evidence budget'); return; }
    state.stdout.push(Buffer.from(bytes));
    state.pending = Buffer.concat([state.pending, bytes]);
    let end;
    while ((end = state.pending.indexOf(10)) >= 0) {
      const line = state.pending.subarray(0, end);
      state.pending = state.pending.subarray(end + 1);
      try {
        if (state.ready) throw new Error('Owned service emitted more than one Ready line');
        state.ready = validateServeReady(new TextDecoder('utf-8', { fatal: true }).decode(line), child.pid, state.role);
      } catch (error) { state.error ??= error; }
    }
    if (state.pending.length > MAX_MESSAGE) state.error ??= new Error('Protocol message exceeds the 16 KiB budget');
  });
  return child;
}

export function serveReady(child) {
  const state = states.get(child);
  if (!state) throw new Error('Unknown owned CLI process');
  if (state.error) throw state.error;
  if (!state.ready && state.exit) throw new Error(`Owned CLI exited before Ready: ${stringifyServeJson(state.exit)}`);
  return state.ready;
}

async function bounded(promise, milliseconds, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(label)), milliseconds); })]); }
  finally { clearTimeout(timer); }
}

async function control(state, identityFile) {
  const args = ['owner-stop', '--data-dir', state.data, '--owner-identity', identityFile];
  const child = state.spawnChild(state.executable, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: state.env });
  const record = state.control = {
    pid: child.pid, executable: state.executable, args,
    startedUtc: new Date().toISOString(), stdout: [], stderr: [],
  };
  let bytes = 0;
  for (const stream of ['stdout', 'stderr']) child[stream].on('data', data => {
    bytes += data.length;
    if (bytes <= limit) record[stream].push(Buffer.from(data));
    else record.error ??= 'OwnerStop output exceeds the evidence budget';
  });
  const done = new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('close', (code, signal) => { Object.assign(record, { exitCode: code, signal, endedUtc: new Date().toISOString() }); resolve(); });
  });
  done.catch(() => {});
  await writeFile(path.join(state.evidence, 'control-start.json'), stringifyServeJson({ pid: child.pid, executable: state.executable, args, startedUtc: record.startedUtc }, 2), { flag: 'wx' });
  await bounded(done, 45000, 'OwnerStop delivery outcome unknown after timeout; do not retry or kill');
  if (record.exitCode !== 0 || record.signal || record.error) {
    throw new Error(record.exitCode === 6 ? 'OwnerStop outcome unknown (6); do not retry or kill' : `OwnerStop failed (${record.exitCode ?? record.signal}): ${record.error ?? 'original output retained'}`);
  }
  return validateOwnerStop(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(record.stdout)), state.ready.ownerIdentity);
}

export function stopServe(child) {
  if (!child) return Promise.resolve();
  const state = states.get(child);
  if (!state) return Promise.reject(new Error('Refuse to stop an unowned process'));
  if (state.stopping) return state.stopping;
  state.stopping = (async () => {
    await mkdir(path.dirname(state.evidence), { recursive: true });
    await mkdir(state.evidence, { recursive: false });
    let failure;
    try {
      if (!state.ready) throw new Error('No qualified Ready identity; retained raw output; do not guess identity or kill');
      if (state.exit) throw new Error('Owner exited without a qualified OwnerStop receipt');
      const identityFile = path.join(state.evidence, 'owner-identity.json');
      await writeFile(identityFile, stringifyServeJson(state.ready.ownerIdentity, 2), { flag: 'wx' });
      await writeFile(path.join(state.evidence, 'intent.json'), stringifyServeJson({ expectedOwner: state.ready.ownerIdentity, executable: state.executable, args: ['owner-stop', '--data-dir', state.data, '--owner-identity', identityFile], automaticAttempts: 1 }, 2), { flag: 'wx' });
      state.receipt = await control(state, identityFile);
      const exited = await bounded(state.exited, 15000, 'Acknowledged owner did not exit within the bounded observation; do not retry or kill');
      if (exited.code !== 0 || exited.signal) throw new Error('Acknowledged owner did not actually exit with code zero');
      if (state.error) throw state.error;
      return state.receipt;
    } catch (error) { failure = error; throw error; }
    finally {
      for (const stream of ['stdout', 'stderr']) {
        await writeFile(path.join(state.evidence, `owner.${stream}.bin`), Buffer.concat(state[stream]), { flag: 'wx' });
        if (state.control) await writeFile(path.join(state.evidence, `control.${stream}.bin`), Buffer.concat(state.control[stream]), { flag: 'wx' });
      }
      const record = {
        executable: state.executable, args: state.args, pid: child.pid,
        ready: state.ready, windowsProcessHandleExposed: false,
        startupFaultStopAttempted: false, captureLimitBytes: limit,
        actualStdoutBytes: state.stdoutBytes, actualStderrBytes: state.stderrBytes,
        control: state.control && Object.fromEntries(Object.entries(state.control).filter(([key]) => !['stdout', 'stderr'].includes(key))),
        receipt: state.receipt, actualOwnerExit: state.exit,
        error: failure && String(failure),
        status: failure ? 'FAILED: no lifecycle acceptance; no retry or kill' : 'PASS: exact ownerStop receipt and actual owner exit zero',
      };
      await writeFile(path.join(state.evidence, 'lifecycle.json'), stringifyServeJson(record, 2), { flag: 'wx' });
    }
  })();
  return state.stopping;
}
