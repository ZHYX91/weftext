import Foundation
import XCTest

enum WebKitCallbackWaitFailure: Error, Equatable {
    case timeout(String)
    case unexpectedResult(String)
}

/// Test-only waiting: a probe timeout is an error for its caller, not an issue
/// permanently recorded by XCTestCase before the caller applies its own policy.
@MainActor
enum WebKitCallbackWaiter {
    static func deadline(outer: TimeInterval? = nil) -> TimeInterval {
        min(ProcessInfo.processInfo.systemUptime + 8, outer ?? .infinity)
    }

    static func wait(for expectation: XCTestExpectation, deadline: TimeInterval, phase: String) async throws {
        let remaining = deadline - ProcessInfo.processInfo.systemUptime
        guard remaining > 0 else { throw WebKitCallbackWaitFailure.timeout(phase) }
        let result = await XCTWaiter.fulfillment(of: [expectation], timeout: min(8, remaining))
        switch result {
        case .completed: return
        case .timedOut: throw WebKitCallbackWaitFailure.timeout(phase)
        default: throw WebKitCallbackWaitFailure.unexpectedResult("\(phase): \(result)")
        }
    }
}
