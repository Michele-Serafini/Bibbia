// swift-tools-version: 6.0
import PackageDescription

var products: [Product] = [.library(name: "BibleKit", targets: ["BibleKit"])]
var targets: [Target] = [
    .systemLibrary(name: "CSQLite", pkgConfig: "sqlite3", providers: [.apt(["libsqlite3-dev"])]),
    .target(name: "BibleKit", dependencies: ["CSQLite"],
            resources: [.copy("Resources/Corpora"), .copy("Resources/Notices")]),
    .testTarget(name: "BibleKitTests", dependencies: ["BibleKit"]),
]
#if os(macOS)
products.append(.executable(name: "Bibbia", targets: ["Bibbia"]))
targets.append(.executableTarget(name: "Bibbia", dependencies: ["BibleKit"]))
#endif

let package = Package(
    name: "BibbiaMac",
    platforms: [.macOS(.v13)],
    products: products,
    targets: targets
)
