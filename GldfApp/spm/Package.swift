// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "GldfKit",
    platforms: [
        .iOS(.v13),
        .macOS(.v13),
        .watchOS(.v6),
    ],
    products: [
        .library(
            name: "GldfKit",
            targets: ["GldfKit"]
        ),
    ],
    targets: [
        // Binary FFI target - the framework module name is GldfFfi
        .binaryTarget(
            name: "GldfFfi",
            path: "GldfFfi.xcframework"
        ),

        // Swift bindings target
        .target(
            name: "GldfKit",
            dependencies: ["GldfFfi"],
            path: "Sources/GldfKit"
        ),

        // Tests
        .testTarget(
            name: "GldfKitTests",
            dependencies: ["GldfKit"],
            path: "Tests/GldfKitTests"
        ),
    ]
)
