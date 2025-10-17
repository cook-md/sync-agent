// Build script to embed the public key at compile time
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Read the public key from .signing-key.pem.pub
    // This file should be generated using: cargo packager signer generate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let pubkey_path = PathBuf::from(&manifest_dir).join(".signing-key.pem.pub");

    // Check if CARGO_PACKAGER_PUBLIC_KEY is already set (e.g., in CI/CD)
    if env::var("CARGO_PACKAGER_PUBLIC_KEY").is_err() {
        // If not set, try to read from file
        if pubkey_path.exists() {
            let pubkey = fs::read_to_string(&pubkey_path)
                .expect("Failed to read .signing-key.pem.pub");
            let pubkey = pubkey.trim();

            // Set the environment variable for the compiler
            println!("cargo:rustc-env=CARGO_PACKAGER_PUBLIC_KEY={}", pubkey);
            println!("cargo:rerun-if-changed={}", pubkey_path.display());
        } else {
            // In development, use a dummy key
            // In production/CI, this should be set via environment variable
            eprintln!("Warning: No .signing-key.pem.pub found and CARGO_PACKAGER_PUBLIC_KEY not set");
            eprintln!("Using dummy public key for development builds");
            println!("cargo:rustc-env=CARGO_PACKAGER_PUBLIC_KEY=DUMMY_KEY_FOR_DEV");
        }
    } else {
        // Use the environment variable directly
        let pubkey = env::var("CARGO_PACKAGER_PUBLIC_KEY").unwrap();
        println!("cargo:rustc-env=CARGO_PACKAGER_PUBLIC_KEY={}", pubkey);
    }

    // For Windows, we need to embed resources
    #[cfg(target_os = "windows")]
    {
        // This is handled by embed-resource crate in Cargo.toml
    }
}
