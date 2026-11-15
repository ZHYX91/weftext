import Foundation
import XCTest

final class WebKitCallbackWaiterTests: XCTestCase {
    @MainActor
    func testProbeTimeoutCanBeCaughtWithoutPermanentlyFailingTheCase() async throws {
        let pending = XCTestExpectation(description: "independent pending probe")
        do {
            try await WebKitCallbackWaiter.wait(for: pending,
                deadline: ProcessInfo.processInfo.systemUptime + 0.01, phase: "probe")
            XCTFail("An unfulfilled probe must not report completion")
        } catch {
            XCTAssertEqual(error as? WebKitCallbackWaitFailure, .timeout("probe"))
        }
        let completed = XCTestExpectation(description: "later completed probe")
        completed.fulfill()
        try await WebKitCallbackWaiter.wait(for: completed,
            deadline: ProcessInfo.processInfo.systemUptime + 1, phase: "later probe")
    }

    @MainActor
    func testDirectCallbackTimeoutRemainsAnError() async throws {
        let pending = XCTestExpectation(description: "independent direct action")
        do {
            try await WebKitCallbackWaiter.wait(for: pending,
                deadline: ProcessInfo.processInfo.systemUptime + 0.01, phase: "direct action")
            XCTFail("A direct action timeout must propagate to its caller")
        } catch {
            XCTAssertEqual(error as? WebKitCallbackWaitFailure, .timeout("direct action"))
        }
    }

    @MainActor
    func testExpiredOuterDeadlineCannotStartAnotherWait() async throws {
        let pending = XCTestExpectation(description: "expired outer readiness")
        let expired = ProcessInfo.processInfo.systemUptime - 1
        XCTAssertEqual(WebKitCallbackWaiter.deadline(outer: expired), expired)
        do {
            try await WebKitCallbackWaiter.wait(for: pending, deadline: expired, phase: "expired readiness")
            XCTFail("An exhausted outer deadline must fail")
        } catch {
            XCTAssertEqual(error as? WebKitCallbackWaitFailure, .timeout("expired readiness"))
        }
    }
}
