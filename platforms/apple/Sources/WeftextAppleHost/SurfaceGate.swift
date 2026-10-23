import Foundation

/// OS/transport failures only. Core's typed reply remains an uninterpreted byte sequence.
public enum AppleHostFailure: String, Error, Sendable {
    case invalidEnvelope, invalidOrigin, retiredSurface, documentAlreadyActive, tooManyCalls, unknownCall
    case invalidAsset, missingAsset, replyBudget, fileAccessDenied, staleBookmark
    case notRegularFile, inputBudget, fileReadFailed, coordinationFailed
}

public struct SurfaceTicket: Equatable, Hashable, Sendable {
    public let surface: UUID
    public let generation: UUID
    public let call: UUID
}

/// A UI delivery lease; this does not prove Core authorization, persistence or safe closure.
public struct SurfaceGate: Sendable {
    public let surface: UUID
    public private(set) var generation: UUID
    public private(set) var active = false
    private var documentNavigationPending = false
    private var calls: Set<UUID> = []
    public static let maximumCalls = 64

    public init(surface: UUID = UUID()) {
        self.surface = surface
        generation = UUID()
    }

    /// Called only when the composition root deliberately loads the trusted entry document.
    public mutating func beginDocument() -> UUID {
        generation = UUID()
        calls.removeAll()
        active = true
        documentNavigationPending = true
        return generation
    }

    public mutating func retire() {
        active = false
        documentNavigationPending = false
        calls.removeAll()
    }

    /// One scheduled entry navigation only. Reload/back/JS navigation cannot retire author input.
    public mutating func consumeDocumentNavigation(_ url: URL) -> Bool {
        guard active, documentNavigationPending, url == AppleAppOrigin.entry else { return false }
        documentNavigationPending = false
        return true
    }

    public mutating func admit(generation: UUID) throws -> SurfaceTicket {
        guard active, self.generation == generation else { throw AppleHostFailure.retiredSurface }
        guard calls.count < Self.maximumCalls else { throw AppleHostFailure.tooManyCalls }
        let id = UUID()
        calls.insert(id)
        return SurfaceTicket(surface: surface, generation: generation, call: id)
    }

    /// Consumes delivery once. A rejected late reply never cancels an accepted Core operation.
    public mutating func finish(_ ticket: SurfaceTicket) -> Bool {
        guard active, ticket.surface == surface, ticket.generation == generation else { return false }
        return calls.remove(ticket.call) != nil
    }
}

public enum AppleAppOrigin {
    public static let scheme = "weftext-bundle"
    public static let host = "app"
    public static let entry = URL(string: "weftext-bundle://app/index.html")!

    public static func accepts(_ url: URL) -> Bool {
        guard let parts = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return false }
        return parts.scheme == scheme && parts.host == host && parts.port == nil &&
            parts.user == nil && parts.password == nil
    }

    public static func assetPath(_ url: URL) throws -> String {
        guard accepts(url), let parts = URLComponents(url: url, resolvingAgainstBaseURL: false),
              parts.query == nil, parts.fragment == nil else { throw AppleHostFailure.invalidOrigin }
        let encoded = parts.percentEncodedPath
        guard encoded.hasPrefix("/"), !encoded.contains("%") else { throw AppleHostFailure.invalidAsset }
        let path = String(encoded.dropFirst())
        guard validAssetPath(path) else { throw AppleHostFailure.invalidAsset }
        return path
    }

    public static func validAssetPath(_ path: String) -> Bool {
        guard !path.isEmpty, path.utf8.count <= 1024, !path.contains("\\"),
              path.unicodeScalars.allSatisfy({ $0.value >= 0x21 && $0.value <= 0x7e }),
              !path.contains("%"), !path.contains("?"), !path.contains("#") else { return false }
        return path.split(separator: "/", omittingEmptySubsequences: false)
            .allSatisfy { !$0.isEmpty && $0 != "." && $0 != ".." }
    }
}
