// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "DictaMacOS",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "DictaMacOS", targets: ["DictaMacOS"])
    ],
    targets: [
        .target(
            name: "WhisperBridge",
            publicHeadersPath: "include",
            cxxSettings: [.unsafeFlags([
                "-I", "Vendor/whisper.cpp/include",
                "-I", "Vendor/whisper.cpp/ggml/include"
            ])]
        ),
        .executableTarget(
            name: "DictaMacOS",
            dependencies: ["WhisperBridge"],
            linkerSettings: [.unsafeFlags([
                "-L", ".build/whisper/lib",
                "-lwhisper", "-lggml", "-lggml-base", "-lggml-cpu",
                "-framework", "Accelerate"
            ])]
        ),
        .testTarget(name: "DictaMacOSTests", dependencies: ["DictaMacOS"])
    ],
    cxxLanguageStandard: .cxx17
)
