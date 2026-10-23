package org.weftext.android.host

import java.net.URI
import java.util.UUID
import java.util.concurrent.atomic.AtomicBoolean
import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.CodingErrorAction

/** UI delivery generation only. It is never a Core identity or authorization. */
class SurfaceGuard {
    val generation: String = UUID.randomUUID().toString()
    val objectName: String = "weftextNative_" + generation.replace("-", "")
    private val active = AtomicBoolean(true)
    fun retire() { active.set(false) }
    fun accepts(origin: String, mainFrame: Boolean): Boolean =
        active.get() && mainFrame && assetOrigin(origin)
    fun current(): Boolean = active.get()

    companion object {
        const val ORIGIN = "https://appassets.androidplatform.net"
        const val MAX_MESSAGE_BYTES = 96 * 1024 * 1024
        const val MAX_REPLY_BYTES = 256 * 1024 * 1024
        fun assetOrigin(value: String): Boolean = runCatching {
            val uri = URI(value)
            uri.scheme == "https" && uri.host == "appassets.androidplatform.net" &&
                uri.port == -1 && uri.rawUserInfo == null && uri.rawFragment == null &&
                uri.rawQuery == null && (uri.rawPath.isNullOrEmpty() || uri.rawPath == "/")
        }.getOrDefault(false)

        fun assetPath(value: String): String? = runCatching {
            val uri = URI(value)
            if (uri.scheme != "https" || uri.host != "appassets.androidplatform.net" ||
                uri.port != -1 || uri.rawUserInfo != null || uri.rawQuery != null || uri.rawFragment != null)
                return null
            val path = uri.rawPath ?: return null
            if (!path.startsWith('/') || path.any { !(it.isLetterOrDigit() || it in "/-._") } ||
                path.split('/').any { it == "." || it == ".." }) return null
            path.removePrefix("/").takeIf { it.isNotEmpty() }
        }.getOrNull()
    }
}
internal fun strictUtf8(value: String): ByteArray {
    val encoded = Charsets.UTF_8.newEncoder().onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT).encode(CharBuffer.wrap(value))
    return ByteArray(encoded.remaining()).also { encoded.get(it) }
}
internal fun strictText(value: ByteArray): String = Charsets.UTF_8.newDecoder()
    .onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT)
    .decode(ByteBuffer.wrap(value)).toString()
