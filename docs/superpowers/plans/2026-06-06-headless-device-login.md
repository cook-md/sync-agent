# Headless Device-Code Login Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OAuth 2.0 device-authorization-grant (RFC 8628) login to `cook-sync login` so users can authenticate on a headless server or in Docker, with no browser.

**Architecture:** A new `src/auth/device_flow.rs` module holds the RFC 8628 client (request a device code, poll for a token). `AuthManager::device_login()` drives it from the terminal and converges on the existing `set_session()`. `cook-sync login` gains a `--headless` flag; a pure `prefer_device_flow()` helper picks browser vs device flow (flag override + auto-detection of Docker / Linux-no-display). The token-response state machine is factored into a pure function so it is unit-testable with no network.

**Tech Stack:** Rust, tokio, reqwest, serde, thiserror, tokio-util (CancellationToken), `open`. All already dependencies — no new crates.

**Reference implementation:** cookcli `src/server/sync/device_flow.rs` and `src/login.rs` (PR cook-md/CookCLI#347), at `/Users/alexeydubovskoy/cooklang/CookCLI`. Server endpoints confirmed in cook.md `web/config/routes.rb` (`namespace :oauth` → `device/code`, `device/token`, at site root, not `/api`).

**Spec:** `docs/superpowers/specs/2026-06-06-headless-device-login-design.md`

---

## File Structure

- **Create** `src/auth/device_flow.rs` — RFC 8628 client: request/response structs, `request_device_code`, the pure `interpret_token_response` state-machine step, `poll_for_token`, `client_name`, `server_host_label`. Owns all device-flow networking and protocol logic.
- **Create** `src/auth/device_flow_test.rs` — unit tests for `interpret_token_response`, `client_name`, `server_host_label`.
- **Modify** `src/auth/mod.rs` — declare the module; add `AuthManager::device_login()`.
- **Modify** `src/main.rs` — add `headless` flag to `Commands::Login`, add pure `prefer_device_flow()` helper + its tests, dispatch in `login()`.

---

## Task 1: Device-flow protocol types and response interpreter

**Files:**
- Create: `src/auth/device_flow.rs`
- Create: `src/auth/device_flow_test.rs`
- Modify: `src/auth/mod.rs` (add `pub mod device_flow;` near the other `pub mod` lines at the top)

- [ ] **Step 1: Declare the module**

In `src/auth/mod.rs`, the top currently reads:

```rust
pub mod jwt;
pub mod secure_session;
```

Change it to:

```rust
pub mod device_flow;
pub mod jwt;
pub mod secure_session;
```

- [ ] **Step 2: Write `src/auth/device_flow.rs` with types and the pure interpreter**

```rust
use std::time::{Duration, Instant};

use log::error;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::{Result, SyncError};

#[cfg(test)]
#[path = "device_flow_test.rs"]
mod device_flow_test;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Response from `POST /oauth/device/code`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
struct DeviceCodeRequest<'a> {
    client_name: &'a str,
}

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    device_code: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenSuccess {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

/// One step of the RFC 8628 token-polling state machine. Pure and
/// network-free so it can be unit-tested.
#[derive(Debug, PartialEq, Eq)]
pub enum PollOutcome {
    /// Got the JWT.
    Token(String),
    /// Keep polling at the current interval.
    Pending,
    /// Increase the polling interval by 5 seconds.
    SlowDown,
    /// User denied authorization (terminal).
    Denied,
    /// Device code expired (terminal).
    Expired,
    /// Unexpected/unparseable response (terminal).
    Bad(String),
}

/// Interpret a `/oauth/device/token` HTTP response body. `is_success` is the
/// HTTP 2xx status; `body` is the raw response text.
pub fn interpret_token_response(is_success: bool, body: &str) -> PollOutcome {
    if is_success {
        return match serde_json::from_str::<TokenSuccess>(body) {
            Ok(ok) => PollOutcome::Token(ok.access_token),
            Err(e) => PollOutcome::Bad(format!("unparseable success body: {e}")),
        };
    }

    let err: TokenError = match serde_json::from_str(body) {
        Ok(e) => e,
        Err(e) => return PollOutcome::Bad(format!("unparseable error body: {e}")),
    };

    match err.error.as_str() {
        "authorization_pending" => PollOutcome::Pending,
        "slow_down" => PollOutcome::SlowDown,
        "access_denied" => PollOutcome::Denied,
        "expired_token" => PollOutcome::Expired,
        other => PollOutcome::Bad(format!("unexpected error code: {other}")),
    }
}

/// Builds the `client_name` sent to cook.md; identifies the device on the
/// approval screen. E.g. `"Cook Sync 0.6.0 (linux/docker)"`.
pub fn client_name() -> String {
    format!(
        "Cook Sync {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        server_host_label()
    )
}

/// Returns "docker" if `/.dockerenv` exists, else "server".
pub fn server_host_label() -> &'static str {
    if std::path::Path::new("/.dockerenv").exists() {
        "docker"
    } else {
        "server"
    }
}
```

- [ ] **Step 3: Write `src/auth/device_flow_test.rs`**

```rust
use super::{client_name, interpret_token_response, PollOutcome};

#[test]
fn success_body_yields_token() {
    let body = r#"{"access_token":"jwt-123"}"#;
    assert_eq!(
        interpret_token_response(true, body),
        PollOutcome::Token("jwt-123".to_string())
    );
}

#[test]
fn success_body_unparseable_is_bad() {
    match interpret_token_response(true, "not json") {
        PollOutcome::Bad(_) => {}
        other => panic!("expected Bad, got {other:?}"),
    }
}

#[test]
fn authorization_pending_yields_pending() {
    let body = r#"{"error":"authorization_pending"}"#;
    assert_eq!(interpret_token_response(false, body), PollOutcome::Pending);
}

#[test]
fn slow_down_yields_slow_down() {
    let body = r#"{"error":"slow_down"}"#;
    assert_eq!(interpret_token_response(false, body), PollOutcome::SlowDown);
}

#[test]
fn access_denied_yields_denied() {
    let body = r#"{"error":"access_denied"}"#;
    assert_eq!(interpret_token_response(false, body), PollOutcome::Denied);
}

#[test]
fn expired_token_yields_expired() {
    let body = r#"{"error":"expired_token"}"#;
    assert_eq!(interpret_token_response(false, body), PollOutcome::Expired);
}

#[test]
fn unknown_error_code_is_bad() {
    let body = r#"{"error":"teapot"}"#;
    match interpret_token_response(false, body) {
        PollOutcome::Bad(msg) => assert!(msg.contains("teapot")),
        other => panic!("expected Bad, got {other:?}"),
    }
}

#[test]
fn error_body_unparseable_is_bad() {
    match interpret_token_response(false, "<html>500</html>") {
        PollOutcome::Bad(_) => {}
        other => panic!("expected Bad, got {other:?}"),
    }
}

#[test]
fn client_name_includes_version_and_os() {
    let name = client_name();
    assert!(name.starts_with("Cook Sync "));
    assert!(name.contains(std::env::consts::OS));
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib auth::device_flow`
Expected: all 9 tests PASS. (`SlowDown`, `poll_for_token`, `request_device_code` are added in Task 2 — the `dead_code`/`unused` warnings for `GRANT_TYPE`, `DeviceCodeRequest`, `TokenRequest`, `DeviceCodeResponse` fields are expected at this point and resolved by Task 2.)

- [ ] **Step 5: Commit**

```bash
git add src/auth/mod.rs src/auth/device_flow.rs src/auth/device_flow_test.rs
git commit -m "feat(auth): add device-flow protocol types and token-response interpreter"
```

---

## Task 2: Network calls — request_device_code and poll_for_token

**Files:**
- Modify: `src/auth/device_flow.rs`

- [ ] **Step 1: Append `request_device_code` and `poll_for_token` to `src/auth/device_flow.rs`**

Add these functions to the file (after `interpret_token_response`):

```rust
/// POST `/oauth/device/code`. `base_url` is the site root (no `/api` suffix).
pub async fn request_device_code(
    client: &reqwest::Client,
    base_url: &str,
    client_name: &str,
) -> Result<DeviceCodeResponse> {
    let url = format!("{base_url}/oauth/device/code");
    let resp = client
        .post(&url)
        .json(&DeviceCodeRequest { client_name })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SyncError::Other(format!(
            "device code request failed: HTTP {status}: {body}"
        )));
    }

    Ok(resp.json::<DeviceCodeResponse>().await?)
}

/// Poll `/oauth/device/token` until approved, denied, expired, or cancelled.
/// Honors `slow_down` (interval += 5s) and the `expires_at` deadline.
pub async fn poll_for_token(
    client: &reqwest::Client,
    base_url: &str,
    device_code: &str,
    mut interval: Duration,
    expires_at: Instant,
    cancel: CancellationToken,
) -> Result<String> {
    let url = format!("{base_url}/oauth/device/token");

    loop {
        if Instant::now() >= expires_at {
            return Err(SyncError::Other(
                "Code expired - run `cook-sync login` again.".to_string(),
            ));
        }

        tokio::select! {
            _ = cancel.cancelled() => {
                return Err(SyncError::Other("Cancelled.".to_string()));
            }
            _ = tokio::time::sleep(interval) => {}
        }

        let resp = client
            .post(&url)
            .json(&TokenRequest {
                grant_type: GRANT_TYPE,
                device_code,
            })
            .send()
            .await?;

        let is_success = resp.status().is_success();
        let body = resp.text().await.unwrap_or_default();

        match interpret_token_response(is_success, &body) {
            PollOutcome::Token(jwt) => return Ok(jwt),
            PollOutcome::Pending => continue,
            PollOutcome::SlowDown => interval += Duration::from_secs(5),
            PollOutcome::Denied => {
                return Err(SyncError::Other("Authorization denied.".to_string()));
            }
            PollOutcome::Expired => {
                return Err(SyncError::Other(
                    "Code expired - run `cook-sync login` again.".to_string(),
                ));
            }
            PollOutcome::Bad(msg) => {
                error!("Device token poll: {msg}");
                return Err(SyncError::Other(format!("Login failed: {msg}")));
            }
        }
    }
}
```

- [ ] **Step 2: Run the full lib test + clippy to verify it compiles cleanly**

Run: `cargo test --lib auth::device_flow && cargo clippy --lib -- -D warnings -A dead_code`
Expected: tests PASS; clippy reports no warnings. (`request_device_code`/`poll_for_token` are now defined but only called in Task 4 — `-A dead_code` suppresses the unused-fn warning until then.)

- [ ] **Step 3: Commit**

```bash
git add src/auth/device_flow.rs
git commit -m "feat(auth): add device-code request and token polling"
```

---

## Task 3: Flow-selection helper `prefer_device_flow`

**Files:**
- Modify: `src/main.rs` (add the function and a `#[cfg(test)] mod tests` block)

- [ ] **Step 1: Write the failing tests**

Add this near the end of `src/main.rs` (the crate root binary):

```rust
#[cfg(test)]
mod login_flow_tests {
    use super::*;

    #[test]
    fn headless_flag_always_prefers_device_flow() {
        assert!(prefer_device_flow_from(
            true,  // headless flag
            false, // docker
            true,  // is_linux
            true,  // has_display (would otherwise force browser)
        ));
    }

    #[test]
    fn docker_prefers_device_flow_without_flag() {
        assert!(prefer_device_flow_from(false, true, true, true));
    }

    #[test]
    fn linux_without_display_prefers_device_flow() {
        assert!(prefer_device_flow_from(false, false, true, false));
    }

    #[test]
    fn linux_with_display_uses_browser() {
        assert!(!prefer_device_flow_from(false, false, true, true));
    }

    #[test]
    fn non_linux_without_flag_uses_browser() {
        // macOS/Windows: always a GUI session, no auto-detection.
        assert!(!prefer_device_flow_from(false, false, false, false));
    }

    #[test]
    fn non_linux_docker_still_prefers_device_flow() {
        assert!(prefer_device_flow_from(false, true, false, true));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --bin cook-sync login_flow_tests`
Expected: FAIL — `cannot find function prefer_device_flow_from`.

(If the binary name differs, use the `name` under `[[bin]]` in `Cargo.toml`; `cargo test login_flow_tests` also works.)

- [ ] **Step 3: Implement `prefer_device_flow` and its pure core**

Add to `src/main.rs` (top-level functions, near `login()`):

```rust
/// Decide whether `cook-sync login` should use the headless device-code flow
/// instead of the browser/loopback flow.
fn prefer_device_flow(headless_flag: bool) -> bool {
    let docker = std::path::Path::new("/.dockerenv").exists();
    let is_linux = cfg!(target_os = "linux");
    let has_display =
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    prefer_device_flow_from(headless_flag, docker, is_linux, has_display)
}

/// Pure decision core, separated for testing.
fn prefer_device_flow_from(
    headless_flag: bool,
    docker: bool,
    is_linux: bool,
    has_display: bool,
) -> bool {
    if headless_flag || docker {
        return true;
    }
    // Auto-detect only on Linux: a Linux session with no X/Wayland display is
    // headless. macOS/Windows always have a GUI, so never auto-switch there.
    is_linux && !has_display
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --bin cook-sync login_flow_tests`
Expected: all 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add prefer_device_flow detection for headless login"
```

---

## Task 4: `AuthManager::device_login()`

**Files:**
- Modify: `src/auth/mod.rs` (add the method to the `impl AuthManager` block; add `use` for the device_flow items)

- [ ] **Step 1: Add the imports**

Near the top of `src/auth/mod.rs`, alongside `use self::jwt::refresh_due;` and `use self::secure_session::SecureSession;`, add:

```rust
use self::device_flow::{client_name, poll_for_token, request_device_code};
```

- [ ] **Step 2: Add the `device_login` method**

Inside `impl AuthManager`, after `browser_login` (and before `handle_callback_request`), add:

```rust
/// Headless OAuth 2.0 device-authorization-grant login (RFC 8628).
/// Prints a user code + verification URL, polls until approval, then saves
/// the session. Works with no browser/display (servers, Docker).
pub async fn device_login(&self) -> Result<()> {
    use std::io::Write;
    use std::time::{Duration, Instant};
    use tokio_util::sync::CancellationToken;

    // Device endpoints live at the site root, not under /api.
    let api_endpoint = self.api.base_url();
    let base_url = api_endpoint
        .strip_suffix("/api")
        .unwrap_or(api_endpoint)
        .to_string();

    let client = reqwest::Client::new();
    let name = client_name();
    let dc = request_device_code(&client, &base_url, &name).await?;

    println!();
    println!("To sign in, open this URL in any browser:");
    println!();
    println!("    {}", dc.verification_uri);
    println!();
    println!("and enter this code:");
    println!();
    println!("    {}", dc.user_code);
    println!();

    // Best-effort: open the browser to the prefilled URL. Harmless if it
    // fails (no display) — the user already has the URL and code above.
    if let Err(e) = open::that(&dc.verification_uri_complete) {
        debug!("Could not open browser automatically: {e}");
    }

    print!("Waiting for authorization");
    let _ = std::io::stdout().flush();

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_for_signal.cancel();
    });

    // Print a dot every second while waiting, so the user sees progress.
    let dot_handle = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        print!(".");
                        let _ = std::io::stdout().flush();
                    }
                }
            }
        })
    };

    let expires_at = Instant::now() + Duration::from_secs(dc.expires_in);
    let interval = Duration::from_secs(dc.interval);

    let jwt_result =
        poll_for_token(&client, &base_url, &dc.device_code, interval, expires_at, cancel.clone())
            .await;

    cancel.cancel();
    dot_handle.abort();
    println!();

    let jwt = jwt_result?;
    self.set_session(jwt)?;

    if let Some(session) = self.get_session() {
        println!(
            "Logged in as {}",
            session.email.unwrap_or(session.user_id)
        );
    }

    Ok(())
}
```

Note: `debug` is already imported in `src/auth/mod.rs` (`use log::{debug, error, info};`).

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: builds successfully. `request_device_code`/`poll_for_token` are now used, so the Task 2 `dead_code` allowance is no longer load-bearing.

- [ ] **Step 4: Commit**

```bash
git add src/auth/mod.rs
git commit -m "feat(auth): add device_login terminal flow"
```

---

## Task 5: Wire `--headless` flag into the CLI and dispatch

**Files:**
- Modify: `src/main.rs` (the `Commands::Login` enum variant; the match arm at ~line 122; the `login()` function signature at ~line 475)

- [ ] **Step 1: Add the flag to the `Login` command variant**

In `src/main.rs`, the variant currently reads:

```rust
    /// Open browser for login
    Login,
```

Change it to:

```rust
    /// Log in (opens a browser; falls back to a device code on headless hosts)
    Login {
        /// Use the headless device-code flow (no browser); for servers / Docker
        #[arg(long)]
        headless: bool,
    },
```

- [ ] **Step 2: Update the match arm**

Find (around line 122):

```rust
        Some(Commands::Login) => login().await,
```

Change to:

```rust
        Some(Commands::Login { headless }) => login(headless).await,
```

- [ ] **Step 3: Update `login()` to accept the flag and dispatch**

Replace the existing `login()` function (around line 475) body's flow-selection. The current function is:

```rust
async fn login() -> Result<()> {
    println!("Opening browser for login...");

    let config = config::Config::new()?;
    let api_endpoint = config::settings::Settings::get_api_endpoint();
    println!("Using API endpoint: {}", api_endpoint);
    let api = api::CookApi::new(api_endpoint)?;
    let auth = auth::AuthManager::new(config.paths(), Arc::new(api))?;

    // Perform browser-based login
    match auth.browser_login().await {
        Ok(()) => {
            println!("Successfully authenticated!");
            if let Some(session) = auth.get_session() {
                println!("Logged in as: {}", session.email.unwrap_or(session.user_id));
            }
        }
        Err(e) => {
            error!("Authentication failed: {e}");
            return Err(e);
        }
    }

    Ok(())
}
```

Replace it with:

```rust
async fn login(headless: bool) -> Result<()> {
    let config = config::Config::new()?;
    let api_endpoint = config::settings::Settings::get_api_endpoint();
    println!("Using API endpoint: {}", api_endpoint);
    let api = api::CookApi::new(api_endpoint)?;
    let auth = auth::AuthManager::new(config.paths(), Arc::new(api))?;

    let use_device = prefer_device_flow(headless);

    let result = if use_device {
        println!("Starting headless device-code login...");
        auth.device_login().await
    } else {
        println!("Opening browser for login...");
        auth.browser_login().await
    };

    match result {
        Ok(()) => {
            // device_login already prints "Logged in as ...".
            if !use_device {
                println!("Successfully authenticated!");
                if let Some(session) = auth.get_session() {
                    println!("Logged in as: {}", session.email.unwrap_or(session.user_id));
                }
            }
        }
        Err(e) => {
            error!("Authentication failed: {e}");
            return Err(e);
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Build and run the full test suite + lint**

Run: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings -A dead_code && cargo fmt --check`
Expected: build OK, all tests PASS, no clippy warnings, formatting clean. (If `cargo fmt --check` fails, run `cargo fmt` and re-check.)

- [ ] **Step 5: Manual smoke test (device flow path)**

Run: `cargo run -- login --headless`
Expected: prints a verification URL and a user code, then "Waiting for authorization" with dots. Ctrl-C prints a clean "Cancelled." and exits non-zero. (Full approval requires a real cook.md account; the goal here is to confirm the code is requested and rendered.)

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add --headless flag and device-flow dispatch to login (#96)"
```

---

## Done

After Task 5, `cook-sync login --headless` performs a device-code login, and `cook-sync login` on a headless Linux host or in Docker auto-selects it. Desktop browser login is unchanged.

**Reminder (out of scope, file separately):** `cook-sync start` still requires a system tray (`src/daemon.rs:244`) and will not run on a truly headless host. Headless *login* is delivered here; headless *daemon* operation is a follow-up issue.
