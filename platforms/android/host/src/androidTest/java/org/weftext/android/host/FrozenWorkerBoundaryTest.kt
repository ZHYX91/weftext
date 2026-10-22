package org.weftext.android.host

import android.content.ComponentName
import android.os.Process
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit

/** Transport/OS probe only. This is not an Engine/Core implementation or product acceptance. */
class ProbeFrozenWorkerService : FrozenWorkerService() {
    override fun executeFrozen(kind: FrozenWorkKind, exactWire: ByteArray): ByteArray {
        val input = strictText(exactWire)
        if (input == "block") { Thread.sleep(30_000); error("probeWatchdogFailed") }
        if (input.startsWith("private:")) {
            val denied = try { File(input.removePrefix("private:")).readBytes(); false }
            catch (_: java.io.IOException) { true } catch (_: SecurityException) { true }
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
}
