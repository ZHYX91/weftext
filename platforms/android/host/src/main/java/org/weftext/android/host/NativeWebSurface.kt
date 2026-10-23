package org.weftext.android.host

import android.annotation.SuppressLint
import android.app.Activity
import android.net.Uri
import android.os.Looper
import android.view.ViewGroup
import android.view.WindowInsets
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.util.concurrent.atomic.AtomicInteger

/** The composition root must supply the real shared owner; there is no default handler. */
interface NativeSurfaceHost {
    fun opened(generation: String, messageObject: String)
    /** Exact UTF-8, never JNI modified-UTF8 String or a normalized/reencoded Core body. */
    fun dispatch(message: ByteArray, reply: (ByteArray) -> Unit)
    /** Call permitted only after the real owner has retained/qualified its current draft. */
    fun requestClose(onPermitted: () -> Unit)
    fun backgrounded()
    fun retired(generation: String)
    fun transportFailure(code: String)
    fun externalLink(uri: Uri)
}

@SuppressLint("SetJavaScriptEnabled")
class NativeWebSurface(private val activity: Activity, private val host: NativeSurfaceHost) {
    val view = WebView(activity)
    private val guard = SurfaceGuard()
    private val inflight = AtomicInteger()
    private val bundled = BundledAssets(activity.assets)
    private var loaded = false
    private var pendingScroll: Pair<Int, Int>? = null
    init {
        check(Looper.myLooper() == Looper.getMainLooper()) { "webViewMainThread" }
        if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER))
            throw IllegalStateException("webMessageListenerUnavailable")
        WebView.setWebContentsDebuggingEnabled(false)
        view.settings.apply {
            javaScriptEnabled = true; domStorageEnabled = true
            allowFileAccess = false; allowContentAccess = false
            mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            setSupportMultipleWindows(false); javaScriptCanOpenWindowsAutomatically = false
            mediaPlaybackRequiresUserGesture = true; builtInZoomControls = true; displayZoomControls = false
        }
        view.webChromeClient = object : WebChromeClient() {
            override fun onPermissionRequest(request: PermissionRequest) { request.deny() }
        }
        view.setDownloadListener { _, _, _, _, _ -> host.transportFailure("unrequestedWebDownload") }
        if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
          WebViewCompat.addWebMessageListener(view, guard.objectName, setOf(SurfaceGuard.ORIGIN)) {
                _, message, origin, mainFrame, reply ->
            if (!guard.accepts(origin.toString(), mainFrame)) return@addWebMessageListener
            val text = try { message.data } catch (_: Exception) { null }
                ?: return@addWebMessageListener
            if (text.length > SurfaceGuard.MAX_MESSAGE_BYTES) {
                host.transportFailure("bridgeInputBudget"); return@addWebMessageListener
            }
            val payload = try { strictUtf8(text) } catch (_: Exception) {
                host.transportFailure("bridgeInputUnicode"); return@addWebMessageListener
            }
            if (payload.size > SurfaceGuard.MAX_MESSAGE_BYTES) {
                host.transportFailure("bridgeInputBudget"); return@addWebMessageListener
            }
            if (inflight.incrementAndGet() > 8) {
                inflight.decrementAndGet(); host.transportFailure("bridgeBusy"); return@addWebMessageListener
            }
            val completed = java.util.concurrent.atomic.AtomicBoolean()
            try {
                host.dispatch(payload) { response ->
                    if (!completed.compareAndSet(false, true)) return@dispatch
                    inflight.decrementAndGet()
                    view.post {
                        if (guard.current()) {
                            if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
                                host.transportFailure("webMessageListenerUnavailable"); return@post
                            }
                            if (response.size > SurfaceGuard.MAX_REPLY_BYTES)
                                host.transportFailure("bridgeReplyBudget")
                            else if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER))
                                try { reply.postMessage(strictText(response)) }
                            catch (_: Exception) { host.transportFailure("bridgeReplyUnicode") }
                        }
                    }
                }
            } catch (_: Exception) {
                if (completed.compareAndSet(false, true)) inflight.decrementAndGet()
                host.transportFailure("bridgeDispatchFailed")
            }
          }
        } else throw IllegalStateException("webMessageListenerUnavailable")
        view.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(v: WebView, request: WebResourceRequest): WebResourceResponse =
                bundled.response(request.url.toString(), request.method)
            override fun shouldOverrideUrlLoading(v: WebView, request: WebResourceRequest): Boolean {
                if (request.isForMainFrame && request.hasGesture() && request.url.scheme == "https" &&
                    !request.url.host.isNullOrEmpty() && request.url.host != "appassets.androidplatform.net" &&
                    request.url.userInfo == null)
                    host.externalLink(request.url)
                return true // No navigation can carry the privileged bridge elsewhere.
            }
            override fun onPageFinished(v: WebView, url: String) {
                if (!guard.current() || SurfaceGuard.assetPath(url) != "index.html") return
                loaded = true
                pendingScroll?.let { (x, y) -> view.scrollTo(x, y) }; pendingScroll = null
                v.evaluateJavascript("window.dispatchEvent(new CustomEvent('weftext-native-surface'," +
                    "{detail:{objectName:'${guard.objectName}',generation:'${guard.generation}'}}));", null)
            }
            override fun onRenderProcessGone(v: WebView, detail: RenderProcessGoneDetail): Boolean {
                retire(); host.transportFailure(if (detail.didCrash()) "webContentCrashed" else "webContentKilled")
                return true
            }
        }
        view.setOnApplyWindowInsetsListener { v, insets ->
            val bars = insets.getInsets(WindowInsets.Type.systemBars())
            val ime = insets.getInsets(WindowInsets.Type.ime())
            v.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, ime.bottom)); insets
        }
        host.opened(guard.generation, guard.objectName)
    }
    fun load() { check(guard.current()); view.loadUrl(SurfaceGuard.ORIGIN + "/index.html") }
    fun requestBack(onPermitted: () -> Unit) {
        if (guard.current()) host.requestClose { view.post { if (guard.current()) onPermitted() } }
    }
    fun backgrounded() { if (guard.current()) host.backgrounded() }
    fun restoreScroll(x: Int, y: Int) {
        if (!guard.current()) return
        if (loaded) view.scrollTo(x, y) else pendingScroll = Pair(x, y)
    }
    fun retire() {
        if (!guard.current()) return
        guard.retire() // retire before removing listeners or notifying the owner.
        try {
            runCatching {
                if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER))
                    WebViewCompat.removeWebMessageListener(view, guard.objectName)
            }
            host.retired(guard.generation)
        } finally { // accepted Core work is not cancelled.
            (view.parent as? ViewGroup)?.removeView(view)
            runCatching { view.stopLoading() }; view.destroy()
        }
    }
}
