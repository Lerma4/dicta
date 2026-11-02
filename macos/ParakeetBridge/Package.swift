// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "DictaParakeetBridge",
    platforms: [.macOS(.v14)],
    products: [.library(name: "DictaParakeetBridge", type: .dynamic, targets: ["DictaParakeetBridge"])],
    dependencies: [.package(url: "https://github.com/FluidInference/FluidAudio.git", exact: "0.17.3")],
    targets: [.target(name: "DictaParakeetBridge", dependencies: [.product(name: "FluidAudio", package: "FluidAudio")])]
)
