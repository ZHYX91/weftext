package org.weftext.android.host

import android.app.Service
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.ParcelFileDescriptor
import android.os.Process
import android.os.RemoteException
import android.os.SharedMemory
import android.system.OsConstants
import java.io.DataInputStream
import java.io.DataOutputStream
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.Semaphore
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

enum class FrozenWorkKind { RENDER, NOTES }
/** PID/UID are read in the owner's Binder callback. Start time is a worker observation. */
data class WorkerIdentity(val pid: Int, val uid: Int, val startedElapsedRealtime: Long)
data class WorkerResult(val identity: WorkerIdentity, val bytes: ByteArray)
private object WorkerWire {
    const val DESCRIPTOR = "org.weftext.frozen-worker.v1"
    const val START = IBinder.FIRST_CALL_TRANSACTION
    const val CANCEL = IBinder.FIRST_CALL_TRANSACTION + 1
    const val IDENTITY = IBinder.FIRST_CALL_TRANSACTION + 2
    const val MAGIC = 0x57465731 // WFW1, transport framing only; Engine JSON is unchanged.
    const val MAX_INPUT = 96 * 1024 * 1024
    const val MAX_OUTPUT = 256 * 1024 * 1024
    fun id(value: String) = UUID.fromString(value).let { it.version() == 4 && it.toString() == value }
}

/** Concrete service must call the real shared frozen Engine codec, not create a Core.
 * Manifest must declare isolatedProcess=true/exported=false. No default executor exists.
 */
abstract class FrozenWorkerService : Service() {
    protected abstract fun executeFrozen(kind: FrozenWorkKind, exactWire: ByteArray): ByteArray
    private val used = AtomicBoolean()
    @Volatile private var activeId: String? = null
    private val execute = Executors.newSingleThreadExecutor()
    private val watchdog = Executors.newSingleThreadScheduledExecutor()
    private val endpoint = object : Binder() {
        @Synchronized
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            if (code == IBinder.INTERFACE_TRANSACTION) { reply?.writeString(WorkerWire.DESCRIPTOR); return true }
            if (code != WorkerWire.START && code != WorkerWire.CANCEL) return super.onTransact(code, data, reply, flags)
            require(reply != null && flags and IBinder.FLAG_ONEWAY == 0) { "workerSynchronousOnly" }
            data.enforceInterface(WorkerWire.DESCRIPTOR)
            check(Binder.getCallingUid() == applicationInfo.uid && Process.myUid() != applicationInfo.uid) {
                "workerCaller"
            }
            val id = data.readString() ?: error("workerId")
            require(WorkerWire.id(id)) { "workerId" }
            if (code == WorkerWire.CANCEL) {
                if (activeId == null && used.compareAndSet(false, true)) activeId = id
                require(data.dataAvail() == 0 && activeId == id) { "workerCancelIdentity" }
                reply.writeNoException()
                terminateSoon()
                return true
            }
            val kind = FrozenWorkKind.valueOf(data.readString() ?: error("workerKind"))
            val budget = data.readLong()
            val expected = data.readString() ?: error("workerDigest")
            require(budget in 1..60_000 && expected.matches(Regex("[a-f0-9]{64}"))) { "workerBudget" }
            val observer = data.readStrongBinder() ?: error("workerObserver")
            val memory = data.readTypedObject(SharedMemory.CREATOR) ?: error("workerInput")
            val output = data.readTypedObject(ParcelFileDescriptor.CREATOR) ?: run { memory.close(); error("workerOutput") }
            try {
                require(data.dataAvail() == 0 && memory.size in 1..WorkerWire.MAX_INPUT) { "workerInputBudget" }
                check(used.compareAndSet(false, true)) { "workerAlreadyUsed" }
                activeId = id
                watchdog.schedule({ Process.killProcess(Process.myPid()) }, budget, TimeUnit.MILLISECONDS)
                val observed = Parcel.obtain(); val acknowledged = Parcel.obtain()
                try {
                    observed.writeInterfaceToken(WorkerWire.DESCRIPTOR); observed.writeString(id)
                    observed.writeLong(Process.getStartElapsedRealtime())
                    check(observer.transact(WorkerWire.IDENTITY, observed, acknowledged, 0)) { "workerObserver" }
                    acknowledged.readException()
                } finally { observed.recycle(); acknowledged.recycle() }
                execute.execute {
                    try {
                        val writable = runCatching { memory.mapReadWrite() }.getOrNull()
                        if (writable != null) {
                            SharedMemory.unmap(writable); error("workerInputNotSealed")
                        }
                        val mapped = memory.mapReadOnly()
                        val source = try { ByteArray(mapped.remaining()).also { mapped.get(it) } }
                        finally { SharedMemory.unmap(mapped) }
                        check(hash(source) == expected) { "workerInputChanged" }
                        val result = executeFrozen(kind, source)
                        require(result.size in 1..WorkerWire.MAX_OUTPUT) { "workerReplyBudget" }
                        frame(output, id, 0, result)
                    } catch (_: Exception) {
                        runCatching { frame(output, id, 1, "workerExecutionFailed".toByteArray(Charsets.US_ASCII)) }
                    } finally {
                        memory.close(); runCatching { output.close() }; terminateSoon()
                    }
                }
                reply.writeNoException()
                reply.writeInt(Process.myPid()); reply.writeInt(Process.myUid())
                reply.writeLong(Process.getStartElapsedRealtime())
                return true
            } catch (error: Exception) { memory.close(); output.close(); throw error }
        }
    }
    override fun onBind(intent: Intent): IBinder? {
        // A wrong manifest must never arrange a watchdog that could kill the owner.
        if (Process.myUid() == applicationInfo.uid) return null
        watchdog.schedule({
            // START and idle expiry compete for the same one-shot admission.
            if (used.compareAndSet(false, true)) Process.killProcess(Process.myPid())
        }, 10, TimeUnit.SECONDS)
        return endpoint
    }
    private fun terminateSoon() { watchdog.schedule({ Process.killProcess(Process.myPid()) }, 25, TimeUnit.MILLISECONDS) }
    private fun frame(fd: ParcelFileDescriptor, id: String, status: Int, bytes: ByteArray) {
        DataOutputStream(ParcelFileDescriptor.AutoCloseOutputStream(fd)).use {
            it.writeInt(WorkerWire.MAGIC); it.writeUTF(id); it.writeInt(status)
            it.writeInt(bytes.size); it.writeUTF(hash(bytes)); it.write(bytes); it.flush()
        }
    }
    override fun onDestroy() { execute.shutdownNow(); watchdog.shutdownNow(); super.onDestroy() }
}

class FrozenWorkerJob internal constructor(val result: CompletableFuture<WorkerResult>,
    /** Completed only by this exact Binder's death, never merely by unbind or result delivery. */
    val terminated: CompletableFuture<Unit>, private val cancelAction: () -> Unit) {
    fun cancel() = cancelAction()
}

/** One new isolated OS instance per pure render/notes job. No Core/native author submission here. */
class FrozenWorkerClient(private val context: Context, private val component: ComponentName) {
    private val slots = Semaphore(2)
    fun start(kind: FrozenWorkKind, input: ByteArray, budgetMillis: Long = 60_000): FrozenWorkerJob {
        require(input.size in 1..WorkerWire.MAX_INPUT && budgetMillis in 1..60_000) { "workerBudget" }
        // Freeze caller-owned memory before binding or asynchronously hashing/sending it.
        val frozen = input.copyOf()
        val digest = hash(frozen)
        val io = Executors.newCachedThreadPool()
        val clock = Executors.newSingleThreadScheduledExecutor()
        @Suppress("DEPRECATION")
        val info = context.packageManager.getServiceInfo(component, 0)
        require(component.packageName == context.packageName && !info.exported &&
            info.flags and ServiceInfo.FLAG_ISOLATED_PROCESS != 0) { "workerComponent" }
        check(slots.tryAcquire()) { "workerBusy" }
        val id = UUID.randomUUID().toString()
        val result = CompletableFuture<WorkerResult>()
        val dead = CompletableFuture<Unit>()
        val cancelled = AtomicBoolean()
        val cancelSent = AtomicBoolean()
        val unbound = AtomicBoolean()
        val remote = AtomicReference<IBinder?>()
        val activeReader = AtomicReference<ParcelFileDescriptor?>()
        val observedIdentity = AtomicReference<WorkerIdentity?>()
        val slotReleased = AtomicBoolean()
        fun releaseSlot() { if (slotReleased.compareAndSet(false, true)) slots.release() }
        val observer = object : Binder() {
            override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
                if (code != WorkerWire.IDENTITY) return super.onTransact(code, data, reply, flags)
                require(reply != null && flags and IBinder.FLAG_ONEWAY == 0) { "workerObserver" }
                data.enforceInterface(WorkerWire.DESCRIPTOR)
                require(data.readString() == id) { "workerIdentity" }
                val identity = WorkerIdentity(Binder.getCallingPid(), Binder.getCallingUid(), data.readLong())
                require(identity.pid > 0 && identity.uid != context.applicationInfo.uid &&
                    identity.startedElapsedRealtime >= 0 && data.dataAvail() == 0) { "workerIdentity" }
                check(observedIdentity.compareAndSet(null, identity)) { "workerIdentityRepeated" }
                reply.writeNoException(); return true
            }
        }
        lateinit var connection: ServiceConnection
        fun unbind() { if (unbound.compareAndSet(false, true)) runCatching { context.unbindService(connection) } }
        fun sendCancel(binder: IBinder) {
            if (cancelSent.compareAndSet(false, true)) {
                runCatching { io.execute { runCatching {
                    val data = Parcel.obtain(); val reply = Parcel.obtain()
                    try {
                        data.writeInterfaceToken(WorkerWire.DESCRIPTOR); data.writeString(id)
                        check(binder.transact(WorkerWire.CANCEL, data, reply, 0)); reply.readException()
                    } finally { data.recycle(); reply.recycle() }
                } } }
            }
        }
        fun cancel() {
            if (!cancelled.compareAndSet(false, true)) return
            runCatching { activeReader.getAndSet(null)?.close() }
            remote.get()?.let(::sendCancel)
            result.completeExceptionally(IllegalStateException("workerCancelled"))
            // Keep the binding until the exact endpoint's death is observed, including pre-START cancellation.
        }
        connection = object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                remote.set(binder)
                try { binder.linkToDeath({ releaseSlot(); dead.complete(Unit); unbind() }, 0) }
                catch (_: RemoteException) {
                    releaseSlot(); dead.complete(Unit)
                    result.completeExceptionally(IllegalStateException("workerDisconnected")); unbind(); return
                }
                catch (_: Exception) {
                    result.completeExceptionally(IllegalStateException("workerDisconnected"))
                    dead.completeExceptionally(IllegalStateException("workerDeathUnobserved")); unbind(); return
                }
                if (dead.isDone) {
                    result.completeExceptionally(IllegalStateException("workerDisconnected")); unbind(); return
                }
                if (cancelled.get()) { sendCancel(binder); return }
                try { io.execute {
                    var memory: SharedMemory? = null
                    var readEnd: ParcelFileDescriptor? = null
                    var writeEnd: ParcelFileDescriptor? = null
                    try {
                        val shared = SharedMemory.create("weftext-frozen-$id", frozen.size)
                        memory = shared
                        val mapped = shared.mapReadWrite()
                        try { mapped.put(frozen) } finally { SharedMemory.unmap(mapped) }
                        check(shared.setProtect(OsConstants.PROT_READ)) { "workerSeal" }
                        val pipe = ParcelFileDescriptor.createReliablePipe()
                        val reader = pipe[0]; val writer = pipe[1]
                        readEnd = reader; writeEnd = writer
                        activeReader.set(reader)
                        val data = Parcel.obtain(); val reply = Parcel.obtain()
                        val identity: WorkerIdentity
                        try {
                            data.writeInterfaceToken(WorkerWire.DESCRIPTOR); data.writeString(id)
                            data.writeString(kind.name); data.writeLong(budgetMillis); data.writeString(digest)
                            data.writeStrongBinder(observer)
                            data.writeTypedObject(shared, 0); data.writeTypedObject(writer, 0)
                            check(!cancelled.get()) { "workerCancelled" }
                            check(binder.transact(WorkerWire.START, data, reply, 0)) { "workerStart" }
                            reply.readException()
                            identity = WorkerIdentity(reply.readInt(), reply.readInt(), reply.readLong())
                            require(identity == observedIdentity.get() && reply.dataAvail() == 0) { "workerIdentity" }
                        } finally { data.recycle(); reply.recycle() }
                        writer.close(); writeEnd = null; shared.close(); memory = null
                        val bytes = DataInputStream(ParcelFileDescriptor.AutoCloseInputStream(reader)).use { stream ->
                            check(stream.readInt() == WorkerWire.MAGIC && stream.readUTF() == id) { "workerReplyIdentity" }
                            val status = stream.readInt(); val length = stream.readInt(); val digest = stream.readUTF()
                            require(status in 0..1 && length in 1..WorkerWire.MAX_OUTPUT) { "workerReplyBudget" }
                            val payload = ByteArray(length); stream.readFully(payload)
                            check(stream.read() == -1 && hash(payload) == digest) { "workerReplyDigest" }
                            reader.checkError()
                            check(status == 0) { "workerExecutionFailed" }
                            payload
                        }
                        readEnd = null
                        activeReader.compareAndSet(reader, null)
                        result.complete(WorkerResult(identity, bytes))
                    } catch (error: Exception) {
                        result.completeExceptionally(IllegalStateException("workerUnconfirmed", error)); cancel()
                    } finally {
                        memory?.close(); runCatching { readEnd?.close() }; runCatching { writeEnd?.close() }
                        // Keep Binder death observation alive after result; service is one-shot and self-terminates.
                    }
                } } catch (_: java.util.concurrent.RejectedExecutionException) {
                    result.completeExceptionally(IllegalStateException("workerDisconnected")); unbind()
                }
            }
            override fun onServiceDisconnected(name: ComponentName) {
                result.completeExceptionally(IllegalStateException("workerDisconnected"))
            }
            override fun onNullBinding(name: ComponentName) {
                result.completeExceptionally(IllegalStateException("workerNotIsolated")); unbind()
                dead.completeExceptionally(IllegalStateException("workerDeathUnobserved"))
            }
            override fun onBindingDied(name: ComponentName) {
                result.completeExceptionally(IllegalStateException("workerBindingDied")); unbind()
            }
        }
        val bound = try {
            context.bindIsolatedService(Intent().setComponent(component), Context.BIND_AUTO_CREATE,
                "weftext_" + id.replace("-", ""), context.mainExecutor, connection)
        } catch (_: Exception) { false }
        if (!bound) {
            result.completeExceptionally(IllegalStateException("workerBindRejected")); unbound.set(true)
            dead.completeExceptionally(IllegalStateException("workerNotBound"))
        }
        clock.schedule({
            if (!dead.isDone) cancel()
            // A timeout cannot complete an OS-death observation; late Binder death still releases its slot.
            clock.shutdown()
        }, budgetMillis + 10_000, TimeUnit.MILLISECONDS)
        dead.whenComplete { _, _ ->
            // An unconfirmed death does not free an OS-isolation slot for a replacement process.
            if (!bound) releaseSlot()
            clock.shutdown(); io.shutdown()
        }
        return FrozenWorkerJob(result, dead, ::cancel)
    }
}
