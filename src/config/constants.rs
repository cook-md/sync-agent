//! Compile-time configuration constants for the sync agent.
//!
//! These constants are determined at compile time based on the build profile.
//! Debug builds use local development endpoints, while release builds use production endpoints.

/// Endpoint configuration module
pub mod endpoints {

    /// API endpoint for authentication and user management
    #[cfg(all(
        debug_assertions,
        not(feature = "staging"),
        not(feature = "local-prod")
    ))]
    pub const API: &str = "http://localhost:3000/api";

    #[cfg(all(
        not(debug_assertions),
        not(feature = "staging"),
        not(feature = "local-prod")
    ))]
    pub const API: &str = "https://cook.md/api";

    #[cfg(all(feature = "staging", not(feature = "local-prod")))]
    pub const API: &str = "https://staging.cook.md/api";

    #[cfg(all(feature = "local-prod", not(feature = "staging")))]
    pub const API: &str = "https://cook.md/api";

    // Fallback for when both features are enabled
    #[cfg(all(feature = "staging", feature = "local-prod"))]
    pub const API: &str = "https://cook.md/api";

    /// Sync server endpoint for recipe synchronization
    #[cfg(all(
        debug_assertions,
        not(feature = "staging"),
        not(feature = "local-prod")
    ))]
    pub const SYNC: &str = "http://localhost:8000";

    #[cfg(all(
        not(debug_assertions),
        not(feature = "staging"),
        not(feature = "local-prod")
    ))]
    pub const SYNC: &str = "https://cook.md/api";

    #[cfg(all(feature = "staging", not(feature = "local-prod")))]
    pub const SYNC: &str = "https://staging.cook.md/api";

    #[cfg(all(feature = "local-prod", not(feature = "staging")))]
    pub const SYNC: &str = "https://cook.md/api";

    // Fallback for when both features are enabled
    #[cfg(all(feature = "staging", feature = "local-prod"))]
    pub const SYNC: &str = "https://cook.md/api";

    // BASE constant only for tests - not used in production code
    #[cfg(test)]
    #[cfg(debug_assertions)]
    pub const BASE: &str = "http://localhost:3000";

    #[cfg(test)]
    #[cfg(not(debug_assertions))]
    pub const BASE: &str = "https://cook.md";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoints_defined() {
        // Ensure all required endpoints are defined
        // These are compile-time constants, so just verify they exist
        let _ = endpoints::BASE;
        let _ = endpoints::API;
        let _ = endpoints::SYNC;
    }

    #[test]
    fn test_endpoints_valid_urls() {
        // Basic validation that endpoints look like URLs
        assert!(endpoints::BASE.starts_with("http://") || endpoints::BASE.starts_with("https://"));
        assert!(endpoints::API.starts_with("http://") || endpoints::API.starts_with("https://"));
        assert!(endpoints::SYNC.starts_with("http://") || endpoints::SYNC.starts_with("https://"));
    }
}
