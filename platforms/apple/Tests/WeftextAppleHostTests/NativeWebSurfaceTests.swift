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
        calls.append(Call(bytes: originalEnvelope, surface: surface))
        defer { completed += 1 }
        if hold {
            return try await withCheckedThrowingContinuation { pending.append($0) }
        }
        return reply
    }

    func finishPending() {
        let held = pending
        pending.removeAll()
        for continuation in held { continuation.resume(returning: reply) }
    }
}

final class NativeWebSurfaceTests: XCTestCase {
    private enum Failure: Error { case timeout(String) }

    @MainActor
    private func makeSurface(_ host: RecordingHost) throws -> NativeWebSurface {
        #if os(macOS)
        _ = NSApplication.shared
        #elseif os(iOS)
        // xcodebuild must provide the actual UIKit XCTest runner; do not substitute AppKit or skip.
        _ = UIApplication.shared
        #endif
        let html = Data("<!doctype html><html><head><meta charset=\"utf-8\"></head><body>Bridge fixture</body></html>".utf8)
        let assets = try BundledAssets(files: ["index.html": AppleAsset(bytes: html, mimeType: "text/html")])
        let surface = NativeWebSurface(host: host, assets: assets, dataStore: .nonPersistent())
        try surface.loadTrustedDocument()
        return surface
    }

    /// Completion-handler form gives each real WebKit call its own XCTest deadline, including Promises.
    @MainActor
    private func javascript(_ body: String, arguments: [String: Any] = [:],
                            in surface: NativeWebSurface) async throws -> Any? {
        let done = expectation(description: "WebKit call \(UUID().uuidString)")
        var completed = false
        var value: Any?
        var failure: Error?
        surface.webView.callAsyncJavaScript(body, arguments: arguments, in: nil, in: .page) { outcome in
            switch outcome {
            case .success(let result): value = result
            case .failure(let error): failure = error
            }
            completed = true
            done.fulfill()
        }
        await fulfillment(of: [done], timeout: 8)
        guard completed else { throw Failure.timeout("WebKit callback did not complete") }
        if let failure { throw failure }
        return value
    }

    @MainActor
    private func until(_ description: String, condition: () async throws -> Bool) async throws {
        let deadline = ProcessInfo.processInfo.systemUptime + 12
        var lastError: Error?
        while ProcessInfo.processInfo.systemUptime < deadline {
            do { if try await condition() { return } }
            catch { lastError = error }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        throw Failure.timeout("\(description): \(String(describing: lastError))")
    }

    @MainActor
    private func ready(_ surface: NativeWebSurface) async throws {
        try await until("trusted entry readiness") {
            try await self.javascript("return document.readyState === 'complete' && typeof window.weftextNative === 'object';",
                                      in: surface) as? Bool == true
        }
    }

    @MainActor
    func testTrustedMainFramePreservesExactEnvelopeAndReplyBytes() async throws {
        let host = RecordingHost()
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
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
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
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
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
        let witness = UUID().uuidString
        _ = try await javascript("window.documentWitness = witness; return true;",
                                 arguments: ["witness": witness], in: surface)
        var denied: [URL?] = []
        surface.onNavigationRejected = { denied.append($0) }
        surface.webView.reload()
        try await until("native reload rejection") { !denied.isEmpty }
        let afterReload = try await javascript("return window.documentWitness;", in: surface) as? String
        XCTAssertEqual(afterReload, witness)
        for target in [AppleAppOrigin.entry.absoluteString,
                       "weftext-bundle://app/index.html?new-document",
                       "weftext-bundle://app/other.html", "https://example.invalid/"] {
            let before = denied.count
            _ = try await javascript("window.location.assign(target); return true;",
                                     arguments: ["target": target], in: surface)
            try await until("navigation rejection \(target)") { denied.count > before }
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
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
        _ = try await javascript("""
            window.originalReply = 'waiting';
            window.weftextNative.call('e30=').then(
              value => { window.originalReply = 'success:' + value; },
              error => { window.originalReply = String(error); });
            return true;
            """, in: surface)
        try await until("original held admission") { host.calls.count == 1 && host.pending.count == 1 }
        // Keep the old DOM observable: losing its JS context must not be the rejection oracle.
        surface.retire()
        host.finishPending()
        try await until("explicit retired delivery rejection") {
            try await self.javascript("return window.originalReply !== 'waiting';", in: surface) as? Bool == true
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
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
        _ = try await javascript("""
            window.documentWitness = witness;
            window.oldDelivery = 'waiting';
            window.weftextNative.call('e30=').then(
              value => { window.oldDelivery = value; },
              error => { window.oldDelivery = String(error); });
            return true;
            """, arguments: ["witness": UUID().uuidString], in: surface)
        try await until("original host admission") { host.calls.count == 1 && host.pending.count == 1 }
        let originalSurface = host.calls[0].surface
        surface.retire()
        // This fixture has no author input; the production caller still needs its actual close proof.
        try surface.loadTrustedDocument()
        // load() is asynchronous. The old complete document also has weftextNative until commit.
        // Require observable replacement of its JS global before using the new document as an oracle.
        try await until("replacement document commit") {
            try await self.javascript("""
                return typeof window.documentWitness === 'undefined' &&
                  document.readyState === 'complete' && typeof window.weftextNative === 'object';
                """, in: surface) as? Bool == true
        }
        let beforeCompletion = try await javascript("return typeof window.oldDelivery;", in: surface) as? String
        XCTAssertEqual(beforeCompletion, "undefined")
        host.finishPending()
        try await until("accepted original work completion") { host.completed == 1 }
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
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
        let result = try await javascript("""
            const frame = document.createElement('iframe');
            document.body.append(frame);
            try {
              await frame.contentWindow.webkit.messageHandlers.weftext.postMessage({});
              return 'unexpected success';
            } catch (error) { return String(error); }
            """, in: surface) as? String
        // Missing payload would be invalidEnvelope in the main frame. This proves the actual frame gate.
        XCTAssertTrue(result?.contains("invalidOrigin") == true, result ?? "missing subframe rejection")
        XCTAssertTrue(host.calls.isEmpty)
    }

    @MainActor
    func testActualWebKitEnforcesTheBundledConnectPolicy() async throws {
        let host = RecordingHost()
        let surface = try makeSurface(host)
        defer { host.finishPending(); surface.retire() }
        try await ready(surface)
        _ = try await javascript("""
            window.blockedDirectives = [];
            window.addEventListener('securitypolicyviolation', event => {
              window.blockedDirectives.push(event.effectiveDirective);
            });
            window.fetch('https://example.invalid/weftext-policy-test').catch(() => {});
            return true;
            """, in: surface)
        try await until("actual connect-src CSP violation") {
            try await self.javascript("return window.blockedDirectives.includes('connect-src');", in: surface) as? Bool == true
        }
        XCTAssertTrue(host.calls.isEmpty)
    }
}
