import Foundation
import WebKit
import XCTest
#if os(macOS)
import AppKit
#elseif os(iOS)
import UIKit
#endif
@testable import WeftextAppleHost

/// OS bridge fixtures only: this port records bytes, not Core/business success or authorization.
@MainActor
private final class RecordingHost: AppleHostPort {
    struct Call {
        let bytes: Data
        let surface: UUID
    }
    var calls: [Call] = []
    var hold = false
    var completed = 0
    private(set) var pending: [CheckedContinuation<Data, Error>] = []
    let reply = Data(" \r\n{\"fixture\":\"exact reply\"}\r\n".utf8)

    func dispatch(originalEnvelope: Data, surface: UUID) async throws -> Data {
        print("WK fixture host dispatch admitted surface=\(surface) call=\(calls.count + 1)")
        calls.append(Call(bytes: originalEnvelope, surface: surface))
        defer {
            completed += 1
            print("WK fixture host dispatch completed count=\(completed)")
        }
        if hold {
            return try await withCheckedThrowingContinuation { pending.append($0) }
        }
        return reply
    }

    func finishPending() {
        let held = pending
        pending.removeAll()
        print("WK fixture host finishing pending=\(held.count)")
        for continuation in held { continuation.resume(returning: reply) }
    }
}

/// A local test container owns the real window until the semantic assertions finish.
/// It does not alter the production WebView's scheduling policy or navigation delegate.
@MainActor
private final class HostedFixture {
    let surface: NativeWebSurface
    let readinessDeadline = ProcessInfo.processInfo.systemUptime + 12
    #if os(macOS)
    private let window: NSWindow
    private let root: NSViewController
    #elseif os(iOS)
    private let window: UIWindow
    private let root: UIViewController
    #endif

    init(surface: NativeWebSurface) {
        self.surface = surface
        let frame = CGRect(x: 0, y: 0, width: 640, height: 480)
        #if os(macOS)
        _ = NSApplication.shared
        let ownedRoot = NSViewController()
        let ownedWindow = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
        root = ownedRoot
        window = ownedWindow
        root.view = surface.webView
        window.isReleasedWhenClosed = false
        surface.webView.frame = frame
        surface.webView.autoresizingMask = [.width, .height]
        window.contentViewController = root
        window.makeKeyAndOrderFront(nil)
        root.view.layoutSubtreeIfNeeded()
        #elseif os(iOS)
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        let ownedWindow: UIWindow
        if let scene = scenes.first(where: { $0.activationState == .foregroundActive }) ?? scenes.first {
            ownedWindow = UIWindow(windowScene: scene)
        } else {
            // An actual runner without a scene uses UIKit's public legacy window path.
            // The ready gate still requires real attachment/visibility; no foreground state is faked.
            ownedWindow = UIWindow(frame: frame)
        }
        let ownedRoot = UIViewController()
        window = ownedWindow
        root = ownedRoot
        window.frame = frame
        surface.webView.frame = frame
        surface.webView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        root.view = surface.webView
        window.rootViewController = root
        window.makeKeyAndVisible()
        window.layoutIfNeeded()
        root.view.layoutIfNeeded()
        #endif
        print(Self.snapshot(surface, phase: "host-created"))
    }

    var isReady: Bool {
        let view = surface.webView
        guard view.window === window, view.bounds.width > 0, view.bounds.height > 0,
              view.navigationDelegate === surface else { return false }
        #if os(macOS)
        return window.isVisible
        #elseif os(iOS)
        // Key/app/scene state is diagnostic: the XCTest runner may own the key window.
        return !window.isHidden && window.alpha > 0
        #endif
    }

    static func snapshot(_ surface: NativeWebSurface, phase: String) -> String {
        let view = surface.webView
        let common = "WK fixture phase=\(phase) url=\(view.url?.absoluteString ?? "nil") loading=\(view.isLoading) progress=\(view.estimatedProgress) bounds=\(view.bounds) attached=\(view.window != nil) originalDelegate=\(view.navigationDelegate === surface)"
        #if os(macOS)
        return common + " windowVisible=\(view.window?.isVisible ?? false) windowKey=\(view.window?.isKeyWindow ?? false)"
        #elseif os(iOS)
        let scene = (view.window?.windowScene).map { String($0.activationState.rawValue) } ?? "none"
        return common + " windowHidden=\(view.window?.isHidden ?? true) windowKey=\(view.window?.isKeyWindow ?? false) appState=\(UIApplication.shared.applicationState.rawValue) sceneState=\(scene)"
        #endif
    }

    func close() {
        surface.retire()
        print(Self.snapshot(surface, phase: "host-retired"))
        surface.webView.removeFromSuperview()
        #if os(macOS)
        window.orderOut(nil)
        window.contentViewController = nil
        window.close()
        #elseif os(iOS)
        window.isHidden = true
        window.rootViewController = nil
        #endif
    }
}

final class NativeWebSurfaceTests: XCTestCase {
    private enum Failure: Error { case timeout(String) }

    @MainActor
    private func makeFixture(_ host: RecordingHost) throws -> HostedFixture {
        let html = Data("<!doctype html><html><head><meta charset=\"utf-8\"></head><body>Bridge fixture</body></html>".utf8)
        let assets = try BundledAssets(files: ["index.html": AppleAsset(bytes: html, mimeType: "text/html")])
        let surface = NativeWebSurface(host: host, assets: assets, dataStore: .nonPersistent())
        return HostedFixture(surface: surface)
    }

    /// Completion-handler form gives each real WebKit call its own XCTest deadline, including Promises.
    @MainActor
    private func javascript(_ body: String, arguments: [String: Any] = [:],
                            in surface: NativeWebSurface, phase: String = "action",
                            deadline outerDeadline: TimeInterval? = nil) async throws -> Any? {
        let deadline = WebKitCallbackWaiter.deadline(outer: outerDeadline)
        guard deadline > ProcessInfo.processInfo.systemUptime else { throw WebKitCallbackWaitFailure.timeout(phase) }
        let done = XCTestExpectation(description: "WebKit \(phase) \(UUID().uuidString)")
        var completed = false
        var value: Any?
        var failure: Error?
        print(HostedFixture.snapshot(surface, phase: "\(phase)-begin"))
        surface.webView.callAsyncJavaScript(body, arguments: arguments, in: nil, in: .page) { outcome in
            switch outcome {
            case .success(let result): value = result
            case .failure(let error): failure = error
            }
            completed = true
            done.fulfill()
        }
        do {
            try await WebKitCallbackWaiter.wait(for: done, deadline: deadline, phase: phase)
        } catch {
            print(HostedFixture.snapshot(surface, phase: "\(phase)-callback-failed: \(error)"))
            throw error
        }
        guard completed else { throw Failure.timeout("WebKit callback did not complete") }
        if let failure { throw failure }
        print(HostedFixture.snapshot(surface, phase: "\(phase)-completed"))
        return value
    }

    @MainActor
    private func until(_ description: String, deadline supplied: TimeInterval? = nil,
                       condition: (TimeInterval) async throws -> Bool) async throws {
        let deadline = supplied ?? ProcessInfo.processInfo.systemUptime + 12
        while ProcessInfo.processInfo.systemUptime < deadline {
            // Only a normal false result is retryable; callback errors/timeouts remain failures.
            if try await condition(deadline) { return }
            let remaining = deadline - ProcessInfo.processInfo.systemUptime
            if remaining > 0 { try await Task.sleep(nanoseconds: UInt64(min(0.02, remaining) * 1_000_000_000)) }
        }
        throw Failure.timeout(description)
    }

    @MainActor
    private func ready(_ fixture: HostedFixture) async throws {
        let surface = fixture.surface
        do {
            try await until("visible native test host", deadline: fixture.readinessDeadline) { _ in fixture.isReady }
            try surface.loadTrustedDocument()
            try await until("trusted load state", deadline: fixture.readinessDeadline) { _ in
                surface.webView.url == AppleAppOrigin.entry && !surface.webView.isLoading && surface.webView.estimatedProgress >= 1
            }
            try await until("trusted entry readiness", deadline: fixture.readinessDeadline) { deadline in
                try await self.javascript("return document.readyState === 'complete' && typeof window.weftextNative === 'object';",
                                          in: surface, phase: "trusted-ready", deadline: deadline) as? Bool == true
            }
        } catch {
            print(HostedFixture.snapshot(surface, phase: "ready-failed: \(error)"))
            throw error
        }
    }

    @MainActor
    func testTrustedMainFramePreservesExactEnvelopeAndReplyBytes() async throws {
        let host = RecordingHost()
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        let request = Data(" \r\n{\"source\":\"\\uD800\\u0000𠮷\"}\r\n".utf8)
        let reply = try await javascript("return await window.weftextNative.call(wire);",
                                         arguments: ["wire": request.base64EncodedString()], in: surface)
        XCTAssertEqual(reply as? String, host.reply.base64EncodedString())
        XCTAssertEqual(host.calls.count, 1)
        XCTAssertEqual(host.calls.first?.bytes, request)
        XCTAssertEqual(host.completed, 1)
        XCTAssertEqual(surface.webView.url, AppleAppOrigin.entry)
    }

    @MainActor
    func testInvalidEnvelopeAndWrongGenerationNeverReachTheHost() async throws {
        let host = RecordingHost()
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        let malformed = try await javascript("""
            try { await window.weftextNative.call('bad_'); return 'unexpected success'; }
            catch (error) { return String(error); }
            """, in: surface) as? String
        XCTAssertTrue(malformed?.contains("invalidEnvelope") == true, malformed ?? "missing rejection")
        let stale = try await javascript("""
            try {
              await window.webkit.messageHandlers.weftext.postMessage({
                generation: '00000000-0000-4000-8000-000000000000', wireBase64: 'e30='
              });
              return 'unexpected success';
            } catch (error) { return String(error); }
            """, in: surface) as? String
        XCTAssertTrue(stale?.contains("retiredSurface") == true, stale ?? "missing rejection")
        XCTAssertTrue(host.calls.isEmpty)
    }

    @MainActor
    func testReloadAndNewDocumentNavigationsKeepTheLiveDocument() async throws {
        let host = RecordingHost()
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        let witness = UUID().uuidString
        _ = try await javascript("window.documentWitness = witness; return true;",
                                 arguments: ["witness": witness], in: surface)
        var denied: [URL?] = []
        surface.onNavigationRejected = { denied.append($0) }
        surface.webView.reload()
        try await until("native reload rejection") { _ in !denied.isEmpty }
        let afterReload = try await javascript("return window.documentWitness;", in: surface) as? String
        XCTAssertEqual(afterReload, witness)
        for target in [AppleAppOrigin.entry.absoluteString,
                       "weftext-bundle://app/index.html?new-document",
                       "weftext-bundle://app/other.html", "https://example.invalid/"] {
            let before = denied.count
            _ = try await javascript("window.location.assign(target); return true;",
                                     arguments: ["target": target], in: surface)
            try await until("navigation rejection \(target)") { _ in denied.count > before }
            let afterNavigation = try await javascript("return window.documentWitness;", in: surface) as? String
            XCTAssertEqual(afterNavigation, witness)
        }
        XCTAssertThrowsError(try surface.loadTrustedDocument())
        XCTAssertTrue(host.calls.isEmpty)
    }

    @MainActor
    func testRetiredSurfaceRejectsReplyInTheStillObservableOriginalDocument() async throws {
        let host = RecordingHost()
        host.hold = true
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        _ = try await javascript("""
            window.originalReply = 'waiting';
            window.weftextNative.call('e30=').then(
              value => { window.originalReply = 'success:' + value; },
              error => { window.originalReply = String(error); });
            return true;
            """, in: surface)
        try await until("original held admission") { _ in host.calls.count == 1 && host.pending.count == 1 }
        // Keep the old DOM observable: losing its JS context must not be the rejection oracle.
        surface.retire()
        host.finishPending()
        try await until("explicit retired delivery rejection") { deadline in
            try await self.javascript("return window.originalReply !== 'waiting';", in: surface,
                                      phase: "retired-delivery", deadline: deadline) as? Bool == true
        }
        let rejection = try await javascript("return window.originalReply;", in: surface) as? String
        XCTAssertTrue(rejection?.contains("retiredSurface") == true, rejection ?? "missing original rejection")
        XCTAssertFalse(rejection?.hasPrefix("success:") == true)
        XCTAssertEqual(host.calls.count, 1)
        XCTAssertEqual(host.completed, 1)
    }

    @MainActor
    func testAcceptedHostWorkFinishesAfterRetirementWithoutEnteringNewDocument() async throws {
        let host = RecordingHost()
        host.hold = true
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        _ = try await javascript("""
            window.documentWitness = witness;
            window.oldDelivery = 'waiting';
            window.weftextNative.call('e30=').then(
              value => { window.oldDelivery = value; },
              error => { window.oldDelivery = String(error); });
            return true;
            """, arguments: ["witness": UUID().uuidString], in: surface)
        try await until("original host admission") { _ in host.calls.count == 1 && host.pending.count == 1 }
        let originalSurface = host.calls[0].surface
        surface.retire()
        // This fixture has no author input; the production caller still needs its actual close proof.
        try surface.loadTrustedDocument()
        // load() is asynchronous. The old complete document also has weftextNative until commit.
        // Require observable replacement of its JS global before using the new document as an oracle.
        try await until("replacement document commit") { deadline in
            guard !surface.webView.isLoading else { return false }
            return try await self.javascript("""
                return typeof window.documentWitness === 'undefined' &&
                  document.readyState === 'complete' && typeof window.weftextNative === 'object';
                """, in: surface, phase: "replacement-commit", deadline: deadline) as? Bool == true
        }
        let beforeCompletion = try await javascript("return typeof window.oldDelivery;", in: surface) as? String
        XCTAssertEqual(beforeCompletion, "undefined")
        host.finishPending()
        try await until("accepted original work completion") { _ in host.completed == 1 }
        let afterCompletion = try await javascript("return typeof window.oldDelivery;", in: surface) as? String
        XCTAssertEqual(afterCompletion, "undefined")
        XCTAssertEqual(host.calls.count, 1)
        host.hold = false
        let reply = try await javascript("return await window.weftextNative.call('e30=');", in: surface) as? String
        XCTAssertEqual(reply, host.reply.base64EncodedString())
        XCTAssertEqual(host.calls.count, 2)
        XCTAssertEqual(host.calls[1].surface, originalSurface)
    }

    @MainActor
    func testActualSubframeCannotDispatchIntoTheHost() async throws {
        let host = RecordingHost()
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        let result = try await javascript("""
            const frame = document.createElement('iframe');
            document.body.append(frame);
            try {
              await frame.contentWindow.webkit.messageHandlers.weftext.postMessage({});
              return 'unexpected success';
            } catch (error) { return String(error); }
            """, in: surface, phase: "actual-subframe-rejection") as? String
        // Missing payload would be invalidEnvelope in the main frame. This proves the actual frame gate.
        XCTAssertTrue(result?.contains("invalidOrigin") == true, result ?? "missing subframe rejection")
        XCTAssertTrue(host.calls.isEmpty)
    }

    @MainActor
    func testActualWebKitEnforcesTheBundledConnectPolicy() async throws {
        let host = RecordingHost()
        let fixture = try makeFixture(host)
        let surface = fixture.surface
        defer { host.finishPending(); fixture.close() }
        try await ready(fixture)
        _ = try await javascript("""
            window.blockedDirectives = [];
            window.addEventListener('securitypolicyviolation', event => {
              window.blockedDirectives.push(event.effectiveDirective);
            });
            window.fetch('https://example.invalid/weftext-policy-test').catch(() => {});
            return true;
            """, in: surface)
        try await until("actual connect-src CSP violation") { deadline in
            try await self.javascript("return window.blockedDirectives.includes('connect-src');", in: surface,
                                      phase: "actual-connect-src", deadline: deadline) as? Bool == true
        }
        XCTAssertTrue(host.calls.isEmpty)
    }
}
