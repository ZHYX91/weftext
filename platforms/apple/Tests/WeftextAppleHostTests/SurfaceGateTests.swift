import Foundation
import XCTest
@testable import WeftextAppleHost

final class SurfaceGateTests: XCTestCase {
    func testRetiredAndReopenedSurfaceNeverAdoptsOldReply() throws {
        var gate = SurfaceGate()
        let first = gate.beginDocument()
        let old = try gate.admit(generation: first)
        gate.retire()
        XCTAssertFalse(gate.finish(old))
        let second = gate.beginDocument()
        XCTAssertNotEqual(first, second)
        XCTAssertThrowsError(try gate.admit(generation: first))
        let current = try gate.admit(generation: second)
        XCTAssertFalse(gate.finish(old))
        XCTAssertTrue(gate.finish(current))
        XCTAssertFalse(gate.finish(current))
    }

    func testCallCapacityIsBoundedAndActualCompletionReleasesOneSlot() throws {
        var gate = SurfaceGate()
        let generation = gate.beginDocument()
        var tickets: [SurfaceTicket] = []
        for _ in 0..<SurfaceGate.maximumCalls { tickets.append(try gate.admit(generation: generation)) }
        XCTAssertThrowsError(try gate.admit(generation: generation))
        XCTAssertTrue(gate.finish(tickets[0]))
        _ = try gate.admit(generation: generation)
        XCTAssertThrowsError(try gate.admit(generation: generation))
    }

    func testSurfaceIdentityDoesNotAliasOtherSurface() throws {
        var first = SurfaceGate(), second = SurfaceGate()
        let a = first.beginDocument(), b = second.beginDocument()
        let ticket = try first.admit(generation: a)
        _ = try second.admit(generation: b)
        XCTAssertFalse(second.finish(ticket))
        XCTAssertTrue(first.finish(ticket))
    }

    func testOnlyOneExplicitEntryNavigationCanReplaceTheDocument() {
        var gate = SurfaceGate()
        XCTAssertFalse(gate.consumeDocumentNavigation(AppleAppOrigin.entry))
        _ = gate.beginDocument()
        XCTAssertFalse(gate.consumeDocumentNavigation(URL(string: "weftext-bundle://app/index.html?x")!))
        XCTAssertFalse(gate.consumeDocumentNavigation(URL(string: "weftext-bundle://app/index.html#x")!))
        XCTAssertTrue(gate.consumeDocumentNavigation(AppleAppOrigin.entry))
        XCTAssertFalse(gate.consumeDocumentNavigation(AppleAppOrigin.entry))
        gate.retire()
        XCTAssertFalse(gate.consumeDocumentNavigation(AppleAppOrigin.entry))
        _ = gate.beginDocument()
        XCTAssertTrue(gate.consumeDocumentNavigation(AppleAppOrigin.entry))
    }

    func testAssetOriginAndExactManifestRejectPathEscapes() throws {
        let html = try AppleAsset(bytes: Data("ok".utf8), mimeType: "text/html")
        let assets = try BundledAssets(files: ["index.html": html])
        XCTAssertEqual(try assets.asset(for: AppleAppOrigin.entry).bytes, html.bytes)
        for bad in ["file:///index.html", "https://app/index.html", "weftext-bundle://other/index.html",
                    "weftext-bundle://app:42/index.html", "weftext-bundle://user@app/index.html",
                    "weftext-bundle://app/%2e%2e/private", "weftext-bundle://app/%69ndex.html",
                    "weftext-bundle://app/index.html?private", "weftext-bundle://app/missing.html"] {
            XCTAssertThrowsError(try assets.asset(for: URL(string: bad)!), bad)
        }
        for bad in ["/index.html", "a//b", "a/../b", "a\\b", "", "a%2fb"] {
            XCTAssertThrowsError(try BundledAssets(files: ["index.html": html, bad: html]), bad)
        }
    }
}
