// swift-tools-version:5.9
import PackageDescription
import Foundation

// SwiftPM package for the Speedrun iOS client (F12).
//
// - `SpeedrunFFI`  : a C-module target that exposes the hand-written C header
//                    from the Rust `speedrun-ffi` crate (single source of truth).
// - `SpeedrunKit`  : an idiomatic Swift wrapper (`SpeedrunEngine`) over that C
//                    ABI, plus a SwiftUI `ReviewView` that drives a review
//                    session on the SHARED Rust engine.
//
// `swift build` compiles the Swift <-> C binding on the host (macOS). The
// actual iOS app links the Rust engine via `SpeedrunFFI.xcframework` (built by
// `./build-xcframework.sh`); see README.md for the Xcode wiring.
//
// Set SPEEDRUN_BUILD_DEMO=1 to also build a host CLI (`speedrun-demo`) that
// statically links the Rust engine and runs a review session. It is opt-in
// because it requires the static lib to be built first:
//     (cd ../speedrun-ffi && cargo build)
//     SPEEDRUN_BUILD_DEMO=1 swift run speedrun-demo

var products: [Product] = [
    .library(name: "SpeedrunKit", targets: ["SpeedrunKit"]),
]

var targets: [Target] = [
    .systemLibrary(name: "SpeedrunFFI", path: "Sources/SpeedrunFFI"),
    .target(
        name: "SpeedrunKit",
        dependencies: ["SpeedrunFFI"],
        path: "Sources/SpeedrunKit"
    ),
]

if ProcessInfo.processInfo.environment["SPEEDRUN_BUILD_DEMO"] != nil {
    products.append(.executable(name: "speedrun-demo", targets: ["speedrun-demo"]))
    targets.append(
        .executableTarget(
            name: "speedrun-demo",
            dependencies: ["SpeedrunKit"],
            path: "Sources/speedrun-demo",
            linkerSettings: [
                .unsafeFlags(["-L", "../speedrun-ffi/target/debug"]),
                .linkedLibrary("speedrun_ffi"),
                .linkedLibrary("iconv"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("IOKit"),
            ]
        )
    )
}

let package = Package(
    name: "SpeedrunKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: products,
    targets: targets
)
