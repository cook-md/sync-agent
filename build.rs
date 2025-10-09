fn main() {
    // Optional: Set build profile configuration flags
    // This allows for more fine-grained control beyond debug_assertions
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Set custom configuration flags based on build profile
    match profile.as_str() {
        "release" => {
            println!("cargo:rustc-cfg=production_build");
        }
        "debug" => {
            println!("cargo:rustc-cfg=development_build");
        }
        _ => {
            // For custom profiles, check if they're release-like
            let opt_level = std::env::var("OPT_LEVEL").unwrap_or_else(|_| "0".to_string());
            if opt_level != "0" {
                println!("cargo:rustc-cfg=production_build");
            } else {
                println!("cargo:rustc-cfg=development_build");
            }
        }
    }

    // Print build information for debugging
    println!("cargo:warning=Building in {profile} mode");

    // Pass SENTRY_DSN from build environment to the binary
    // This allows CI/CD to inject the DSN at build time
    if let Ok(sentry_dsn) = std::env::var("SENTRY_DSN") {
        println!("cargo:rustc-env=SENTRY_DSN_EMBEDDED={}", sentry_dsn);
        println!("cargo:warning=Embedding Sentry DSN in binary");
    }

    // Add Windows-specific configurations
    #[cfg(target_os = "windows")]
    {
        // Ensure we have an application manifest on Windows
        embed_resource::compile("resources/windows/app.rc", embed_resource::NONE);
    }

    // macOS-specific build configurations
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=Cocoa");
        println!("cargo:rustc-link-arg=-framework");
        println!("cargo:rustc-link-arg=AppKit");
    }
}
