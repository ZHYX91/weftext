import Foundation
import WebKit

/// Required composition-root port into the single Rust Host. No default/local HTTP executor exists.
/// The implementation validates the canonical shared envelope and fixed call registry, authorizes
/// before body decoding and returns the exact typed reply bytes. UI retirement is not cancellation.
public protocol AppleHostPort: AnyObject {
    func dispatch(originalEnvelope: Data, surface: UUID) async throws -> Data
}

/// Main-thread owned WKWebView surface, common to AppKit and UIKit containers.
public final class NativeWebSurface: NSObject, WKScriptMessageHandlerWithReply, WKNavigationDelegate {
    public let webView: WKWebView
    private let host: AppleHostPort
    private let assets: AssetSchemeHandler
    private var gate = SurfaceGate()
    private static let handler = "weftext"
    private static let maximumWire = 96 * 1024 * 1024
    private static let maximumReply = 256 * 1024 * 1024

    public init(host: AppleHostPort, assets: BundledAssets, dataStore: WKWebsiteDataStore) {
        precondition(Thread.isMainThread)
        self.host = host
        let handler = AssetSchemeHandler(assets)
        self.assets = handler
        let config = WKWebViewConfiguration()
        config.websiteDataStore = dataStore
        config.defaultWebpagePreferences.allowsContentJavaScript = true
        config.preferences.javaScriptCanOpenWindowsAutomatically = false
        config.setURLSchemeHandler(handler, forURLScheme: AppleAppOrigin.scheme)
        webView = WKWebView(frame: .zero, configuration: config)
        super.init()
        webView.navigationDelegate = self
    }

    /// The caller must first obtain the actual Core input-close proof before replacing a live document.
    public func loadTrustedDocument() throws {
        precondition(Thread.isMainThread)
        guard !gate.active else { throw AppleHostFailure.documentAlreadyActive }
        let generation = gate.beginDocument().uuidString.lowercased()
        let controller = webView.configuration.userContentController
        controller.removeScriptMessageHandler(forName: Self.handler, contentWorld: .page)
        controller.removeAllUserScripts()
        controller.addScriptMessageHandler(self, contentWorld: .page, name: Self.handler)
        let script = """
        Object.defineProperty(window, 'weftextNative', { configurable: false, writable: false,
          value: Object.freeze({ call: (wireBase64) => window.webkit.messageHandlers.weftext.postMessage({
            generation: '\(generation)', wireBase64 }) }) });
        """
        controller.addUserScript(WKUserScript(source: script, injectionTime: .atDocumentStart,
                                               forMainFrameOnly: true, in: .page))
        webView.load(URLRequest(url: AppleAppOrigin.entry))
    }

    /// Detaches UI delivery. This cannot release the Core owner or prove any pending write stopped.
    public func retire() {
        precondition(Thread.isMainThread)
        gate.retire()
        let controller = webView.configuration.userContentController
        controller.removeScriptMessageHandler(forName: Self.handler, contentWorld: .page)
        controller.removeAllUserScripts()
        webView.stopLoading()
    }

    public func userContentController(_ userContentController: WKUserContentController,
        didReceive message: WKScriptMessage, replyHandler: @escaping (Any?, String?) -> Void) {
        precondition(Thread.isMainThread)
        let origin = message.frameInfo.securityOrigin
        guard message.name == Self.handler, message.webView === webView, message.frameInfo.isMainFrame,
              origin.protocol == AppleAppOrigin.scheme, origin.host == AppleAppOrigin.host, origin.port == 0,
              let frameURL = message.frameInfo.request.url, AppleAppOrigin.accepts(frameURL) else {
            replyHandler(nil, AppleHostFailure.invalidOrigin.rawValue); return
        }
        guard let body = message.body as? [String: Any], Set(body.keys) == ["generation", "wireBase64"],
              let generationString = body["generation"] as? String,
              let generation = UUID(uuidString: generationString),
              generation.uuidString.lowercased() == generationString,
              let base64 = body["wireBase64"] as? String,
              base64.utf8.count <= (Self.maximumWire + 2) / 3 * 4,
              let bytes = Data(base64Encoded: base64), !bytes.isEmpty,
              bytes.count <= Self.maximumWire, bytes.base64EncodedString() == base64 else {
            replyHandler(nil, AppleHostFailure.invalidEnvelope.rawValue); return
        }
        let ticket: SurfaceTicket
        do { ticket = try gate.admit(generation: generation) }
        catch { replyHandler(nil, failureCode(error)); return }
        // Keep the accepted Host operation alive independently of this surface's delivery lease.
        let host = self.host
        Task { @MainActor [weak self] in
            let outcome: Result<Data, Error>
            do { outcome = .success(try await host.dispatch(originalEnvelope: bytes, surface: ticket.surface)) }
            catch { outcome = .failure(error) }
            guard let self, self.gate.finish(ticket) else {
                replyHandler(nil, AppleHostFailure.retiredSurface.rawValue); return
            }
            switch outcome {
            case .success(let reply):
                guard reply.count <= Self.maximumReply else {
                    replyHandler(nil, AppleHostFailure.replyBudget.rawValue); return
                }
                replyHandler(reply.base64EncodedString(), nil)
            case .failure(let error): replyHandler(nil, self.failureCode(error))
            }
        }
    }

    private func failureCode(_ error: Error) -> String {
        (error as? AppleHostFailure)?.rawValue ?? AppleHostFailure.unknownCall.rawValue
    }

    public func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
        precondition(Thread.isMainThread)
        guard navigationAction.targetFrame?.isMainFrame == true,
              navigationAction.navigationType == .other,
              let url = navigationAction.request.url,
              gate.consumeDocumentNavigation(url) else {
            decisionHandler(.cancel); return
        }
        decisionHandler(.allow)
    }

    public func webViewWebContentProcessDidTerminate(_ webView: WKWebView) { retire() }
}

private final class AssetSchemeHandler: NSObject, WKURLSchemeHandler {
    private let assets: BundledAssets
    init(_ assets: BundledAssets) { self.assets = assets }
    func webView(_ webView: WKWebView, start urlSchemeTask: WKURLSchemeTask) {
        do {
            guard urlSchemeTask.request.httpMethod == "GET", let url = urlSchemeTask.request.url else {
                throw AppleHostFailure.invalidAsset
            }
            let asset = try assets.asset(for: url)
            let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: [
                "Content-Type": asset.mimeType,
                "Content-Length": String(asset.bytes.count),
                "X-Content-Type-Options": "nosniff",
                "Content-Security-Policy": "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'none'; worker-src 'self' blob:; frame-src 'none'; base-uri 'none'; form-action 'none'"
            ])!
            urlSchemeTask.didReceive(response)
            urlSchemeTask.didReceive(asset.bytes)
            urlSchemeTask.didFinish()
        } catch { urlSchemeTask.didFailWithError(error) }
    }
    func webView(_ webView: WKWebView, stop urlSchemeTask: WKURLSchemeTask) {}
}
