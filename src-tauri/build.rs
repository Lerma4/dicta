fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/platform/macos.m")
            .flag("-fobjc-arc")
            .compile("dicta_macos");
        for framework in ["AppKit", "AVFoundation", "ApplicationServices"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=src/platform/macos.m");
    }
    tauri_build::build();
}
