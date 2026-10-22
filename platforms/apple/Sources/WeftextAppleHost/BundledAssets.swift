import Foundation

public struct AppleAsset: Sendable {
    public let bytes: Data
    public let mimeType: String

    public init(bytes: Data, mimeType: String) throws {
        guard !bytes.isEmpty, bytes.count <= 64 * 1024 * 1024,
              Self.mimeTypes.contains(mimeType) else { throw AppleHostFailure.invalidAsset }
        self.bytes = bytes
        self.mimeType = mimeType
    }

    private static let mimeTypes: Set<String> = [
        "text/html", "text/css", "text/javascript", "application/json", "application/wasm",
        "image/svg+xml", "image/png", "image/jpeg", "image/webp", "image/gif",
        "font/woff2", "font/ttf", "font/otf"
    ]
}

/// Exact immutable product bytes supplied by the build's composition root, never author files.
public struct BundledAssets: Sendable {
    private let files: [String: AppleAsset]
    public static let maximumTotalBytes = 512 * 1024 * 1024

    public init(files: [String: AppleAsset]) throws {
        guard !files.isEmpty, files.count <= 4096, files["index.html"]?.mimeType == "text/html",
              files.keys.allSatisfy(AppleAppOrigin.validAssetPath) else { throw AppleHostFailure.invalidAsset }
        var total = 0
        for asset in files.values {
            guard asset.bytes.count <= Self.maximumTotalBytes - total else { throw AppleHostFailure.invalidAsset }
            total += asset.bytes.count
        }
        self.files = files
    }

    public func asset(for url: URL) throws -> AppleAsset {
        let path = try AppleAppOrigin.assetPath(url)
        guard let asset = files[path] else { throw AppleHostFailure.missingAsset }
        return asset
    }
}
