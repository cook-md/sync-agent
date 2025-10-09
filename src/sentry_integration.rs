use log::{debug, error, info};
use once_cell::sync::OnceCell;
use sentry::{ClientInitGuard, ClientOptions};
use std::env;

// Static storage for Sentry guard to keep it alive for the application lifetime
static SENTRY_GUARD: OnceCell<ClientInitGuard> = OnceCell::new();

pub fn init_sentry() {
    // Get Sentry DSN from environment variable or use embedded DSN from build time
    let dsn = env::var("SENTRY_DSN")
        .ok()
        .or_else(|| option_env!("SENTRY_DSN_EMBEDDED").map(String::from));

    if let Some(dsn) = dsn {
        let guard = sentry::init((
            dsn.as_str(),
            ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                environment: Some(
                    if cfg!(debug_assertions) {
                        "development"
                    } else {
                        "production"
                    }
                    .into(),
                ),
                attach_stacktrace: true,
                sample_rate: 1.0,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ));

        // Set up panic handler
        let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            sentry::capture_event(sentry::protocol::Event {
                message: Some(format!("{}", panic_info)),
                level: sentry::Level::Fatal,
                ..Default::default()
            });
            default_panic(panic_info);
        }));

        info!("Sentry error tracking initialized");
        debug!(
            "Sentry DSN source: {}",
            if env::var("SENTRY_DSN").is_ok() {
                "runtime environment"
            } else {
                "build-time embedded"
            }
        );

        // Store the guard using OnceCell to keep Sentry alive without memory leak
        if SENTRY_GUARD.set(guard).is_err() {
            error!("Sentry was already initialized");
        }
    } else {
        debug!("Sentry DSN not configured, error tracking disabled");
    }
}

/// Shutdown Sentry cleanly (call this before application exit if needed)
#[allow(dead_code)]
pub fn shutdown_sentry() {
    if let Some(_guard) = SENTRY_GUARD.get() {
        debug!("Shutting down Sentry");
        // The guard will be properly dropped when taken
        // Note: In practice, we keep the guard alive for the app lifetime
        // This function is mainly for clean shutdown scenarios
    }
}

#[allow(dead_code)]
pub fn capture_message(message: &str, level: sentry::Level) {
    sentry::capture_message(message, level);
}

#[allow(dead_code)]
pub fn configure_scope<F>(f: F)
where
    F: FnOnce(&mut sentry::Scope),
{
    sentry::configure_scope(f);
}

// Helper to add user context
#[allow(dead_code)]
pub fn set_user_context(user_id: Option<String>, email: Option<String>) {
    configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: user_id,
            email,
            ..Default::default()
        }));
    });
}

// Helper to add custom tags
#[allow(dead_code)]
pub fn add_tag(key: &str, value: &str) {
    configure_scope(|scope| {
        scope.set_tag(key, value);
    });
}

// Helper for breadcrumbs
#[allow(dead_code)]
pub fn add_breadcrumb(message: &str, category: &str) {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        message: Some(message.to_string()),
        category: Some(category.to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentry_init_without_dsn() {
        // Should not panic when DSN is not set
        init_sentry();
    }

    #[test]
    fn test_capture_helpers() {
        // These should work even without Sentry initialized
        capture_message("Test message", sentry::Level::Info);
        add_tag("test_key", "test_value");
        add_breadcrumb("Test breadcrumb", "test");
        set_user_context(
            Some("test_user".to_string()),
            Some("test@example.com".to_string()),
        );
    }
}
