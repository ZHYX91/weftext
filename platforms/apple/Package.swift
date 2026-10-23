// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "WeftextAppleHost",
    platforms: [.macOS(.v13), .iOS(.v16)],
    products: [.library(name: "WeftextAppleHost", targets: ["WeftextAppleHost"])],
    targets: [
        .target(name: "WeftextAppleHost"),
        .testTarget(name: "WeftextAppleHostTests", dependencies: ["WeftextAppleHost"])
    ]
)
