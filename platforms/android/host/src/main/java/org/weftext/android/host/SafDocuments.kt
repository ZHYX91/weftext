package org.weftext.android.host

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.system.Os
import android.system.OsConstants
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.atomic.AtomicBoolean

enum class PickerPurpose { OPEN_ORIGINAL, CREATE_ARTIFACT }
class PendingPicker internal constructor(val id: String, val purpose: PickerPurpose,
    val mime: String, val suggestedName: String?) {
    internal val selected = AtomicBoolean()
    fun state(): Bundle = Bundle().apply {
        putString("id", id); putString("purpose", purpose.name)
        putString("mime", mime); putString("name", suggestedName)
    }
    fun intent(): Intent = when (purpose) {
        PickerPurpose.OPEN_ORIGINAL -> Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE); type = mime
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        }
        PickerPurpose.CREATE_ARTIFACT -> Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE); type = mime
            putExtra(Intent.EXTRA_TITLE, suggestedName)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        }
    }
    companion object {
        fun open(originalRequestId: String, mime: String) = checked(originalRequestId, PickerPurpose.OPEN_ORIGINAL, mime, null)
        fun create(originalRequestId: String, mime: String, suggestedName: String) =
            checked(originalRequestId, PickerPurpose.CREATE_ARTIFACT, mime, suggestedName)
        fun restore(state: Bundle): PendingPicker = checked(state.getString("id") ?: error("pickerId"),
            PickerPurpose.valueOf(state.getString("purpose") ?: error("pickerPurpose")),
            state.getString("mime") ?: error("pickerMime"), state.getString("name"))
        private fun checked(id: String, purpose: PickerPurpose, mime: String, name: String?): PendingPicker {
            require(UUID.fromString(id).toString() == id) { "pickerId" }
            require(mime.length in 3..128 && mime.matches(Regex("[A-Za-z0-9.+*-]+/[A-Za-z0-9.+*-]+"))) { "pickerMime" }
            require(purpose != PickerPurpose.CREATE_ARTIFACT || name != null && name.length in 1..255 &&
                name.none { it.isISOControl() || it == '/' || it == '\\' }) { "pickerName" }
            return PendingPicker(id, purpose, mime, name)
        }
    }
}

class DocumentSelection internal constructor(val request: PendingPicker, val uri: Uri,
    val persisted: Boolean, val flags: Int) {
    internal val attempted = AtomicBoolean()
}
class CreatedDocument internal constructor(internal val selection: DocumentSelection) {
    internal val attempted get() = selection.attempted
}
data class OriginalMetadata(val displayName: String?, val mime: String?, val declaredSize: Long?)

/** Pure retained upload input, not a second author store or success ledger. */
class StagedOriginal internal constructor(private val file: File, val length: Long, val sha256: String,
    val metadata: OriginalMetadata) {
    fun verifiedBytes(): ByteArray {
        val bytes = FileInputStream(file).use { boundedRead(it, SafDocuments.MAX_IMPORT) }
        check(bytes.size.toLong() == length && hash(bytes) == sha256) { "originalChanged" }
        return bytes
    }
    /** Native-only recovery locator; do not send it to Server as a capability. */
    fun retainedInputFile(): File = file
}

sealed interface DeliveryResult {
    data class NotSubmitted(val code: String) : DeliveryResult
    data class Unknown(val code: String, val uri: Uri) : DeliveryResult
    data class Verified(val length: Long, val sha256: String, val uri: Uri) : DeliveryResult
}

/** SAF is a URI capability, never a Core directory or filesystem identity. */
class SafDocuments(private val context: Context) {
    private val resolver = context.contentResolver
    fun selected(request: PendingPicker, resultCode: Int, result: Intent?): DocumentSelection? {
        if (resultCode != Activity.RESULT_OK) return null
        val uri = result?.data ?: error("pickerUri")
        require(uri.scheme == "content" && uri.authority != null && uri.fragment == null) { "pickerUri" }
        val allowed = if (request.purpose == PickerPurpose.OPEN_ORIGINAL) Intent.FLAG_GRANT_READ_URI_PERMISSION
            else Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        val flags = result.flags and allowed
        val required = if (request.purpose == PickerPurpose.OPEN_ORIGINAL)
            Intent.FLAG_GRANT_READ_URI_PERMISSION else Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        require(flags and required != 0) { "pickerPermission" }
        check(request.selected.compareAndSet(false, true)) { "pickerAlreadySelected" }
        val persisted = if (result.flags and Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION != 0) {
            try {
                when (flags) {
                    Intent.FLAG_GRANT_READ_URI_PERMISSION ->
                        resolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    Intent.FLAG_GRANT_WRITE_URI_PERMISSION ->
                        resolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION ->
                        resolver.takePersistableUriPermission(uri,
                            Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
                    else -> error("pickerPermission")
                }
                true
            }
            catch (_: SecurityException) { false }
        } else false
        return DocumentSelection(request, uri, persisted, flags)
    }
    fun created(selection: DocumentSelection): CreatedDocument {
        require(selection.request.purpose == PickerPurpose.CREATE_ARTIFACT) { "pickerPurpose" }
        return CreatedDocument(selection)
    }
    fun metadata(selection: DocumentSelection): OriginalMetadata {
        var name: String? = null; var size: Long? = null
        resolver.query(selection.uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use {
            if (it.moveToFirst()) {
                val n = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                val s = it.getColumnIndex(OpenableColumns.SIZE)
                if (n >= 0 && !it.isNull(n)) name = it.getString(n)
                if (s >= 0 && !it.isNull(s)) size = it.getLong(s).takeIf { value -> value >= 0 }
            }
        }
        return OriginalMetadata(name, resolver.getType(selection.uri), size)
    }
    fun freezeOriginal(selection: DocumentSelection): StagedOriginal {
        require(selection.request.purpose == PickerPurpose.OPEN_ORIGINAL) { "pickerPurpose" }
        val info = metadata(selection)
        require(info.declaredSize == null || info.declaredSize <= MAX_IMPORT) { "originalBudget" }
        val anchor = context.noBackupFilesDir.canonicalFile
        val directory = File(anchor, "android-original-inputs")
        if (!directory.exists()) check(directory.mkdir()) { "inputDirectory" }
        check(directory.canonicalFile.parentFile == anchor &&
            OsConstants.S_ISDIR(Os.lstat(directory.path).st_mode)) { "inputDirectory" }
        val destination = File(directory, selection.request.id + ".original")
        val fd = Os.open(destination.path, OsConstants.O_WRONLY or OsConstants.O_CREAT or
            OsConstants.O_EXCL or OsConstants.O_NOFOLLOW, 0x180) // 0600, create-new.
        val digest = MessageDigest.getInstance("SHA-256")
        var count = 0L
        FileOutputStream(fd).use { output ->
            resolver.openInputStream(selection.uri)?.use { input ->
                val buffer = ByteArray(64 * 1024)
                while (true) {
                    val n = input.read(buffer)
                    if (n < 0) break
                    check(n.toLong() <= MAX_IMPORT - count) { "originalBudget" }
                    output.write(buffer, 0, n); digest.update(buffer, 0, n); count += n
                }
            } ?: error("originalUnavailable")
            output.flush(); output.fd.sync()
        }
        val directoryFd = Os.open(directory.path, OsConstants.O_RDONLY or OsConstants.O_NOFOLLOW, 0)
        try {
            check(OsConstants.S_ISDIR(Os.fstat(directoryFd).st_mode)) { "inputDirectory" }
            Os.fsync(directoryFd)
        } finally { Os.close(directoryFd) }
        // Failures deliberately leave partial input evidence; originals are never deleted.
        return StagedOriginal(destination, count, hex(digest.digest()), info)
    }

    /** Caller must durably bind this exact selected URI to its original artifact before any write. */
    fun deliver(target: CreatedDocument, bytes: ByteArray, expectedSha256: String,
        retainTargetBeforeWrite: (DocumentSelection, Long, String) -> Boolean): DeliveryResult {
        val selection = target.selection
        if (bytes.size > MAX_EXPORT) return DeliveryResult.NotSubmitted("artifactBudget")
        val frozen = bytes.copyOf()
        if (hash(frozen) != expectedSha256)
            return DeliveryResult.NotSubmitted("artifactDigest")
        if (!retainTargetBeforeWrite(selection, frozen.size.toLong(), expectedSha256))
            return DeliveryResult.NotSubmitted("targetNotRetained")
        try {
            if (metadata(selection).declaredSize?.let { it > 0 } == true)
                return DeliveryResult.NotSubmitted("targetNotEmpty")
        } catch (_: Exception) { return DeliveryResult.NotSubmitted("targetUnavailable") }
        if (!target.attempted.compareAndSet(false, true))
            return DeliveryResult.NotSubmitted("targetAlreadyAttempted")
        try {
            // Only a target returned by CREATE_DOCUMENT; never resolves or overwrites an input URI.
            resolver.openOutputStream(selection.uri, "w")?.use { it.write(frozen); it.flush() }
                ?: return DeliveryResult.Unknown("providerOutputUnavailable", selection.uri)
            val actual = resolver.openInputStream(selection.uri)?.use { boundedRead(it, MAX_EXPORT.toLong()) }
                ?: return DeliveryResult.Unknown("providerReadbackUnavailable", selection.uri)
            if (actual.size != frozen.size || hash(actual) != expectedSha256)
                return DeliveryResult.Unknown("providerReadbackChanged", selection.uri)
            return DeliveryResult.Verified(frozen.size.toLong(), expectedSha256, selection.uri)
        } catch (_: Exception) {
            return DeliveryResult.Unknown("providerDeliveryUnconfirmed", selection.uri)
        }
    }
    /** Reconcile the retained exact URI after a lost reply/process death. This never writes. */
    fun verifyDelivery(uri: Uri, expectedLength: Long, expectedSha256: String): DeliveryResult {
        require(uri.scheme == "content" && uri.authority != null && uri.fragment == null) { "pickerUri" }
        require(expectedLength in 0..MAX_EXPORT.toLong() && expectedSha256.matches(Regex("[a-f0-9]{64}"))) {
            "artifactDigest"
        }
        return try {
            val bytes = resolver.openInputStream(uri)?.use { boundedRead(it, MAX_EXPORT.toLong()) }
                ?: return DeliveryResult.Unknown("providerReadbackUnavailable", uri)
            if (bytes.size.toLong() == expectedLength && hash(bytes) == expectedSha256)
                DeliveryResult.Verified(expectedLength, expectedSha256, uri)
            else DeliveryResult.Unknown("providerReadbackChanged", uri)
        } catch (_: Exception) { DeliveryResult.Unknown("providerDeliveryUnconfirmed", uri) }
    }
    /** Explicit user revocation only; Activity stop never discards a persistent grant. */
    fun forget(uri: Uri) {
        val grant = resolver.persistedUriPermissions.firstOrNull { it.uri == uri } ?: return
        var flags = 0
        if (grant.isReadPermission) flags = flags or Intent.FLAG_GRANT_READ_URI_PERMISSION
        if (grant.isWritePermission) flags = flags or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        resolver.releasePersistableUriPermission(uri, flags)
    }
    companion object { const val MAX_IMPORT = 16L * 1024 * 1024; const val MAX_EXPORT = 64 * 1024 * 1024 }
}
internal fun boundedRead(input: java.io.InputStream, limit: Long): ByteArray {
    val output = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(64 * 1024)
    while (true) {
        val n = input.read(buffer)
        if (n < 0) break
        require(n.toLong() <= limit - output.size()) { "inputBudget" }
        output.write(buffer, 0, n)
    }
    return output.toByteArray()
}
internal fun hash(bytes: ByteArray): String = hex(MessageDigest.getInstance("SHA-256").digest(bytes))
internal fun hex(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it.toInt() and 255) }
