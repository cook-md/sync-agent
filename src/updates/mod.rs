// Backend modules
mod axo_backend;
mod sparkle_backend;
mod appimage_backend;

// Unified update manager with multiple backends
pub mod update_manager;

// Export the update manager
pub use update_manager::UpdateManager;
