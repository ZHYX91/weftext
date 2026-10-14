import assert from "node:assert/strict";
import { test } from "node:test";
import { backupChunk, backupEnded, backupReceipt, canCommitBackup, canDownloadBackup, decodeBackupJob, type BackupCatalog, type BackupReceipt } from "./hosted-owner-backup-state";
import { hostedCachePrefix } from "./hosted-state";
const request = { requestId: "original" }, digest = "a".repeat(64), artifact = { digest, length: 3, filename: "backup.zip", contentType: "application/zip" };
const catalog: BackupCatalog = { enabled: true, rights: { read: true, export: true, download: true }, scope: "completeSelectedLibraryRecoveryMaterials", automaticRestore: false };
const prepared: BackupReceipt = { requestId: "original", status: "prepared", planId: "plan", commitAllowed: true, automaticRestore: false, summary: "全体私稿和原操作" };
const committed: BackupReceipt = { ...prepared, status: "committed", planId: null, commitAllowed: false, artifact, deliveryUnavailable: false };
test("backup independent rights and fresh authority are required before commit or download", () => {
  assert.equal(canCommitBackup({ request, receipt: prepared }, false, catalog), false); assert.equal(canCommitBackup({ request, receipt: prepared }, true, catalog), true);
  assert.equal(canDownloadBackup({ request, receipt: committed }, false, catalog), false); assert.equal(canDownloadBackup({ request, receipt: committed }, true, catalog), true);
  assert.equal(canDownloadBackup({ request, receipt: committed }, true, { ...catalog, rights: { read: true, export: true, download: false } }), false);
});
test("restart expiry and config filtering preserve original fact but never reuse cached artifact", () => {
  const expired = backupReceipt(request, { ...committed, artifact: null, summary: null, deliveryUnavailable: true }, committed); assert.equal(expired.status, "committed"); assert.equal(canDownloadBackup({ request, receipt: expired }, true, catalog), false); assert.equal(expired.artifact, null);
  assert.throws(() => backupReceipt(request, prepared, committed), /事实/); assert.throws(() => backupReceipt(request, { ...prepared, requestId: "other" }), /原请求/);
  assert.throws(() => backupReceipt(request, { ...committed, artifact: { ...artifact, length: 80 * 1024 * 1024 + 1 } }), /工件/);
  assert.equal(backupEnded("resolved"), true); assert.equal(backupEnded("notRegistered"), false);
});
test("original commit plan and same ID survive reload, generations partition backup and content caches", () => {
  const job = decodeBackupJob(JSON.stringify({ request, receipt: prepared, phase: "commit", commit: { requestId: "original", planId: "plan" } })); assert.equal(job.commit!.planId, "plan");
  assert.throws(() => decodeBackupJob(JSON.stringify({ request, commit: { requestId: "new", planId: "plan" } })), /确认请求/);
  assert.notEqual(hostedCachePrefix("owner-backup", "deploy", "alice", 3), hostedCachePrefix("owner-backup", "deploy", "alice", 4)); assert.notEqual(hostedCachePrefix("content-import", "deploy", "alice", 3), hostedCachePrefix("content-import", "deploy", "bob", 3));
});
test("chunks verify original digest, exact offsets, total size, progress and purpose-specific layout", () => {
  const chunk = { requestId: "original", digest, offset: 0, nextOffset: 3, length: 3, done: true, base64: btoa("zip") }; assert.equal(new TextDecoder().decode(backupChunk(request, artifact, 0, chunk)), "zip");
  for (const mutation of [{ digest: "b".repeat(64) }, { offset: 1 }, { nextOffset: 2 }, { length: 4 }, { done: false }, { base64: "" }]) assert.throws(() => backupChunk(request, artifact, 0, { ...chunk, ...mutation }));
  assert.equal(backupChunk(request, { ...artifact, length: 0 }, 0, { ...chunk, nextOffset: 0, length: 0, base64: "" }).length, 0);
  const large = { ...artifact, length: 65537 }; assert.throws(() => backupChunk(request, large, 0, { ...chunk, length: 65537, nextOffset: 65537, base64: btoa("a".repeat(65537)) }), /分块/);
});
