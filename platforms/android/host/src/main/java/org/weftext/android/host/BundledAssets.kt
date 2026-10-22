package org.weftext.android.host

import android.content.res.AssetManager
import android.webkit.WebResourceResponse
import java.io.ByteArrayInputStream
import java.net.URLConnection

/** APK assets only; never resolves a content://, file:// or author path. */
class BundledAssets(private val assets: AssetManager) {
    fun response(url: String, method: String): WebResourceResponse {
        val path = SurfaceGuard.assetPath(url)
        if (method != "GET" || path == null) return denied()
        return try {
            val stream = assets.open("weftext/" + path, AssetManager.ACCESS_STREAMING)
            val mime = when {
                path.endsWith(".js") -> "text/javascript"
                path.endsWith(".css") -> "text/css"
                path.endsWith(".wasm") -> "application/wasm"
                else -> URLConnection.guessContentTypeFromName(path) ?: "application/octet-stream"
            }
            WebResourceResponse(mime, null, 200, "OK", HEADERS, stream)
        } catch (_: java.io.IOException) { denied() }
    }
    companion object {
        private val HEADERS = mapOf(
            "Content-Security-Policy" to "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; " +
                "img-src 'self' data: blob:; font-src 'self' data:; worker-src 'self' blob:; " +
                "connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
            "X-Content-Type-Options" to "nosniff", "Cache-Control" to "no-store",
            "Referrer-Policy" to "no-referrer"
        )
        fun denied() = WebResourceResponse("text/plain", "UTF-8", 403, "Blocked", HEADERS,
            ByteArrayInputStream(ByteArray(0)))
    }
}
