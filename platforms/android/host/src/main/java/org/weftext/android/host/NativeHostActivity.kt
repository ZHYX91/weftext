package org.weftext.android.host

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher

/** The Application composition root owns Core. Activity destruction never closes it. */
abstract class NativeHostActivity : Activity() {
    protected abstract fun attachOwner(restoredProcessView: Boolean): NativeSurfaceHost
    private var surface: NativeWebSurface? = null
    private var removeBackCallback: (() -> Unit)? = null
    // API 30–34 require the explicit platform edge-to-edge switch; insets own keyboard layout.
    @Suppress("DEPRECATION")
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        window.setDecorFitsSystemWindows(false)
        window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING)
        val current = NativeWebSurface(this, attachOwner(state != null))
        surface = current
        setContentView(current.view)
        if (Build.VERSION.SDK_INT >= 33) {
            val callback = OnBackInvokedCallback { current.requestBack { finish() } }
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT, callback)
            removeBackCallback = { onBackInvokedDispatcher.unregisterOnBackInvokedCallback(callback) }
        }
        current.load() // Never restore WebView history, native bridge objects or old Session IDs.
        if (state != null) current.view.post {
            current.restoreScroll(state.getInt("viewScrollX"), state.getInt("viewScrollY"))
        }
    }
    @Suppress("DEPRECATION")
    @Deprecated("Platform callback on API 30–32; newer Android uses OnBackInvokedDispatcher")
    override fun onBackPressed() { surface?.requestBack { finish() } }
    override fun onSaveInstanceState(state: Bundle) {
        surface?.view?.let { state.putInt("viewScrollX", it.scrollX); state.putInt("viewScrollY", it.scrollY) }
        // No authored source, cached permission, Core receipt or Session in the Android Bundle.
        super.onSaveInstanceState(state)
    }
    override fun onStop() { surface?.backgrounded(); super.onStop() }
    override fun onDestroy() {
        removeBackCallback?.invoke(); removeBackCallback = null
        surface?.retire(); surface = null
        super.onDestroy()
    }
}
