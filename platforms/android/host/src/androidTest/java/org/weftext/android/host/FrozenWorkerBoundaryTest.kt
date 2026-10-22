package org.weftext.android.host

import android.content.ComponentName
import android.os.Process
import android.os.Bundle
import android.os.Build
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit
import java.security.MessageDigest
import org.json.JSONObject

/** Transport/OS probe only. This is not an Engine/Core implementation or product acceptance. */
class ProbeFrozenWorkerService : FrozenWorkerService() {
    override fun executeFrozen(kind: FrozenWorkKind, exactWire: ByteArray): ByteArray {
        val input = strictText(exactWire)
        if (input == "block") { Thread.sleep(30_000); error("probeWatchdogFailed") }
        if (input == "dieAfterIdentity") {
            // executeFrozen runs only after the real owner Binder callback has acknowledged PID/UID.
            Process.killProcess(Process.myPid()); error("probeSelfKillFailed")
        }
        if (input.startsWith("large:")) {
            val size = input.removePrefix("large:").toInt()
            require(size in 1..32 * 1024 * 1024)
            return ByteArray(size) { (it % 251).toByte() }
        }
        if (input == "facts") return strictUtf8(systemFacts().toString())
        if (input.startsWith("private:")) {
            val denied = try {
                val file = Os.open(input.removePrefix("private:"), OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW, 0)
                Os.close(file); false
            } catch (error: ErrnoException) {
                error.errno == OsConstants.EACCES || error.errno == OsConstants.EPERM
            }
            check(denied) { "probePrivateFileReadable" }
            val networkDenied = try {
                val socket = Os.socket(OsConstants.AF_INET, OsConstants.SOCK_STREAM, 0)
                Os.close(socket); false
            } catch (error: ErrnoException) {
                error.errno == OsConstants.EACCES || error.errno == OsConstants.EPERM
            }
            check(networkDenied) { "probeNetworkNotDenied" }
        }
        return exactWire // Explicitly tests byte transport, not frozen Engine JSON execution.
    }
}

class FrozenWorkerBoundaryTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private fun client() = FrozenWorkerClient(context,
        ComponentName(context.packageName, ProbeFrozenWorkerService::class.java.name))

    @Test fun isolatedUidCannotReadOwnerOrCreateNetworkSocket() {
        val marker = File(context.noBackupFilesDir, "worker-probe-${UUID.randomUUID()}")
        marker.outputStream().use { it.write("owner-private".toByteArray()) }
        try {
            val bytes = strictUtf8("private:" + marker.absolutePath)
            val job = client().start(FrozenWorkKind.RENDER, bytes, 5_000)
            val result = job.result.get(10, TimeUnit.SECONDS)
            assertArrayEquals(bytes, result.bytes)
            assertNotEquals(Process.myUid(), result.identity.uid)
            assertNotEquals(Process.myPid(), result.identity.pid)
            job.terminated.get(10, TimeUnit.SECONDS)
        } finally { check(marker.delete()) { "ownedProbeCleanup" } }
    }

    @Test fun cancellationBeforeStartStillObservesExactDeath() {
        val job = client().start(FrozenWorkKind.NOTES, strictUtf8("block"), 5_000)
        job.cancel()
        assertThrows(Exception::class.java) { job.result.get(10, TimeUnit.SECONDS) }
        job.terminated.get(10, TimeUnit.SECONDS)
    }

    @Test fun workerDeadlineTerminatesWithoutKillingOwner() {
        val pid = Process.myPid()
        val job = client().start(FrozenWorkKind.RENDER, strictUtf8("block"), 200)
        assertThrows(Exception::class.java) { job.result.get(10, TimeUnit.SECONDS) }
        job.terminated.get(10, TimeUnit.SECONDS)
        assertEquals(pid, Process.myPid())
    }

    @Test fun completeLargeReplySurvivesNormalWorkerDeath() {
        val size = 32 * 1024 * 1024
        val job = client().start(FrozenWorkKind.RENDER, strictUtf8("large:$size"), 30_000)
        val actual = job.result.get(40, TimeUnit.SECONDS)
        assertEquals(size, actual.bytes.size)
        val expected = MessageDigest.getInstance("SHA-256")
        val chunk = ByteArray(251 * 1024) { (it % 251).toByte() }
        var remaining = size
        while (remaining > 0) {
            val n = minOf(remaining, chunk.size); expected.update(chunk, 0, n); remaining -= n
        }
        assertEquals(hex(expected.digest()), hash(actual.bytes))
        job.terminated.get(10, TimeUnit.SECONDS)
        reportProbe("completeLargeReply", actual.identity, JSONObject().put("bytes", size)
            .put("sha256", hash(actual.bytes)).put("terminatedObserved", true))
    }

    @Test fun immediateObservedDeathDoesNotLeakSlotsAcrossThreeJobs() {
        val sameClient = client()
        repeat(3) {
            val job = sameClient.start(FrozenWorkKind.NOTES, strictUtf8("dieAfterIdentity"), 5_000)
            val identity = job.identity.get(10, TimeUnit.SECONDS)
            assertNotEquals(Process.myUid(), identity.uid)
            assertThrows(Exception::class.java) { job.result.get(10, TimeUnit.SECONDS) }
            job.terminated.get(10, TimeUnit.SECONDS)
            reportProbe("immediateObservedDeath", identity,
                JSONObject().put("iteration", it + 1).put("terminatedObserved", true).put("resultSucceeded", false))
        }
    }

    @Test fun reportAppPrivateFilesystemAndActualIsolatedProcessFacts() {
        val owner = systemFacts()
        val directory = File(context.noBackupFilesDir, "os-facts-${UUID.randomUUID()}")
        val original = File(directory, "before"); val renamed = File(directory, "after")
        val link = File(directory, "symlink")
        Os.mkdir(directory.path, 0x1c0) // 0700, exact owned fixture only.
        try {
            val fd = Os.open(original.path, OsConstants.O_RDWR or OsConstants.O_CREAT or OsConstants.O_EXCL or OsConstants.O_NOFOLLOW, 0x180)
            try {
                val before = Os.fstat(fd)
                Os.fsync(fd); Os.rename(original.path, renamed.path)
                val after = Os.fstat(fd)
                assertEquals(before.st_dev, after.st_dev); assertEquals(before.st_ino, after.st_ino)
                owner.put("renamePreservedOpenFd", true)
            } finally { Os.close(fd) }
            Os.symlink(renamed.path, link.path)
            val rejected = try {
                val unexpected = Os.open(link.path, OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW, 0)
                Os.close(unexpected); false
            } catch (error: ErrnoException) { error.errno == OsConstants.ELOOP }
            assertTrue(rejected); owner.put("noFollowRejectedSymlink", rejected)
            val ancestors = org.json.JSONArray()
            var ancestor: File? = directory
            while (ancestor != null) {
                val current = ancestor
                try {
                    val handle = Os.open(current.path, OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW, 0)
                    try {
                        val value = Os.fstat(handle)
                        assertTrue(OsConstants.S_ISDIR(value.st_mode))
                        if (current == directory || current == context.noBackupFilesDir) Os.fsync(handle)
                        ancestors.put(JSONObject().put("path", current.path).put("dev", value.st_dev)
                            .put("ino", value.st_ino).put("uid", value.st_uid).put("gid", value.st_gid).put("mode", value.st_mode))
                    } finally { Os.close(handle) }
                } catch (error: ErrnoException) {
                    if (current == directory || current == context.noBackupFilesDir) throw error
                    ancestors.put(JSONObject().put("path", current.path).put("openErrno", error.errno))
                }
                ancestor = current.parentFile
            }
            val fs = Os.statvfs(directory.path)
            owner.put("ancestors", ancestors).put("statvfsFsid", fs.f_fsid).put("statvfsFlags", fs.f_flag)
                .put("birthQuery", "notExposedByJavaOsStat").put("qualifiedCoreIdentity", false)
            val job = client().start(FrozenWorkKind.RENDER, strictUtf8("facts"), 5_000)
            val reply = job.result.get(10, TimeUnit.SECONDS)
            val isolated = JSONObject(strictText(reply.bytes))
            assertEquals(reply.identity.uid, isolated.getInt("uid"))
            assertEquals(reply.identity.pid, isolated.getInt("pid"))
            job.terminated.get(10, TimeUnit.SECONDS)
            val report = JSONObject().put("owner", owner).put("isolated", isolated)
            InstrumentationRegistry.getInstrumentation().sendStatus(2,
                Bundle().apply { putString("weftextOsFacts", report.toString()) })
        } finally {
            if (link.exists() || java.nio.file.Files.isSymbolicLink(link.toPath())) java.nio.file.Files.delete(link.toPath())
            if (original.exists()) java.nio.file.Files.delete(original.toPath())
            if (renamed.exists()) java.nio.file.Files.delete(renamed.toPath())
            java.nio.file.Files.delete(directory.toPath())
        }
    }
}

private fun systemFacts(): JSONObject {
    val status = File("/proc/self/status").readLines().filter { line ->
        listOf("Uid:", "Gid:", "CapInh:", "CapPrm:", "CapEff:", "CapBnd:", "CapAmb:", "NoNewPrivs:").any(line::startsWith)
    }
    val selinux = try { File("/proc/self/attr/current").readText().trimEnd('\u0000', '\n') }
        catch (_: Exception) { "unavailable" }
    return JSONObject().put("pid", Process.myPid()).put("uid", Process.myUid()).put("sdk", Build.VERSION.SDK_INT)
        .put("fingerprint", Build.FINGERPRINT).put("procStatus", org.json.JSONArray(status)).put("selinux", selinux)
}

private fun reportProbe(case: String, identity: WorkerIdentity, details: JSONObject) {
    details.put("case", case).put("pid", identity.pid).put("uid", identity.uid)
        .put("startedElapsedRealtime", identity.startedElapsedRealtime)
    InstrumentationRegistry.getInstrumentation().sendStatus(2,
        Bundle().apply { putString("weftextWorkerProbe", details.toString()) })
}
