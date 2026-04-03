// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NexusMessenger",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "NexusMessenger", targets: ["NexusMessenger"]),
    ],
    dependencies: [
        // No external dependencies - use system frameworks only for max auditability
        // TODO: To enable production Kyber1024, add:
        // .package(url: "https://github.com/open-quantum-safe/liboqs-swift", from: "0.8.0")
    ],
    targets: [
        .target(
            name: "NexusMessenger",
            path: "Sources/NexusMessenger"
        ),
        .testTarget(
            name: "NexusMessengerTests",
            dependencies: ["NexusMessenger"],
            path: "Tests"
        ),
    ]
)
