# Daily JWT Rotation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the daemon attempt a JWT refresh at least every 24 hours, decided by wall-clock elapsed time, so a sleeping laptop never misses the refresh window.

**Architecture:** Persist a `last_refresh` unix-epoch timestamp in the keyring alongside the session. Every token acquisition (login or refresh) funnels through `AuthManager::set_session`, which stamps the timestamp. The hourly refresh task replaces its bare near-expiry check with a pure `refresh_due` helper that fires when near expiry (safety net), when the timestamp is unknown, or when ≥24h have elapsed. The comparison reads `Utc::now()` each tick, so a tick firing late after wake still observes the elapsed time and refreshes.

**Tech Stack:** Rust, tokio, chrono, the existing `KeyringStore` trait + `MockKeyring`.

---

### Task 1: `refresh_due` decision helper

A pure function deciding whether to attempt a refresh now. Lives in `jwt.rs` next to `should_refresh`, tested in the existing `jwt_test.rs`.

**Files:**
- Modify: `src/auth/jwt.rs` (add free function after the `impl JwtToken` block, around line 78)
- Test: `src/auth/jwt_test.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/auth/jwt_test.rs`:

```rust
#[test]
fn test_refresh_due_when_last_refresh_unknown() {
    use crate::auth::jwt::refresh_due;
    let now = chrono::Utc::now().timestamp();
    // Token far from expiry (10 days out)
    let claims = json!({ "uid": "user", "exp": now + 10 * 86400 });
    let jwt = JwtToken::from_string(create_jwt_with_claims(claims)).unwrap();

    assert!(
        refresh_due(&jwt, None, now),
        "Unknown last_refresh should be treated as due"
    );
}

#[test]
fn test_refresh_due_recent_refresh_far_from_expiry() {
    use crate::auth::jwt::refresh_due;
    let now = chrono::Utc::now().timestamp();
    let claims = json!({ "uid": "user", "exp": now + 10 * 86400 });
    let jwt = JwtToken::from_string(create_jwt_with_claims(claims)).unwrap();

    // Refreshed 1 hour ago
    assert!(
        !refresh_due(&jwt, Some(now - 3600), now),
        "Recently refreshed token far from expiry should not be due"
    );
}

#[test]
fn test_refresh_due_after_24h() {
    use crate::auth::jwt::refresh_due;
    let now = chrono::Utc::now().timestamp();
    let claims = json!({ "uid": "user", "exp": now + 10 * 86400 });
    let jwt = JwtToken::from_string(create_jwt_with_claims(claims)).unwrap();

    // Refreshed 25 hours ago
    assert!(
        refresh_due(&jwt, Some(now - 25 * 3600), now),
        "Token refreshed over 24h ago should be due"
    );
}

#[test]
fn test_refresh_due_near_expiry_overrides_recent_refresh() {
    use crate::auth::jwt::refresh_due;
    let now = chrono::Utc::now().timestamp();
    // Token expires in 30 minutes (within should_refresh's 1h window)
    let claims = json!({ "uid": "user", "exp": now + 1800 });
    let jwt = JwtToken::from_string(create_jwt_with_claims(claims)).unwrap();

    // Even though refreshed 1 minute ago, near-expiry safety net wins
    assert!(
        refresh_due(&jwt, Some(now - 60), now),
        "Near-expiry safety net should force a refresh"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib auth::jwt 2>&1 | tail -20`
Expected: FAIL — `cannot find function 'refresh_due' in module 'crate::auth::jwt'`

- [ ] **Step 3: Write the implementation**

In `src/auth/jwt.rs`, add this free function after the closing `}` of the `impl JwtToken` block (after line 78):

```rust
/// Decide whether a token refresh should be attempted now.
///
/// Returns true when the token is near expiry (existing safety net), when the
/// last-refresh time is unknown, or when at least 24 hours have elapsed since the
/// last refresh (daily rotation). The 24h check uses wall-clock `now` so it stays
/// correct across machine sleep.
pub fn refresh_due(jwt: &JwtToken, last_refresh: Option<i64>, now: i64) -> bool {
    const DAILY_SECS: i64 = 24 * 60 * 60;

    if jwt.should_refresh() {
        return true;
    }

    match last_refresh {
        None => true,
        Some(ts) => now - ts >= DAILY_SECS,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib auth::jwt 2>&1 | tail -20`
Expected: PASS — all `test_refresh_due_*` tests plus the existing jwt tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/auth/jwt.rs src/auth/jwt_test.rs
git commit -m "feat(auth): add refresh_due daily-rotation decision helper"
```

---

### Task 2: Persist `last_refresh` in the keyring

Add a `last_refresh` key to the secure store with save/load helpers, and ensure `delete` clears it. Tested via the existing `MockKeyring`.

**Files:**
- Modify: `src/auth/secure_session.rs`
- Test: `src/auth/secure_session_test.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/auth/secure_session_test.rs`:

```rust
#[test]
fn test_last_refresh_save_and_load() {
    let mock = MockKeyring::new();

    assert!(
        SecureSession::save_last_refresh_with_mock(&mock, 1_700_000_000).is_ok(),
        "Should save last_refresh timestamp"
    );

    let loaded = SecureSession::load_last_refresh_with_mock(&mock);
    assert!(loaded.is_ok(), "Should load last_refresh without error");
    assert_eq!(loaded.unwrap(), Some(1_700_000_000));
}

#[test]
fn test_last_refresh_missing_returns_none() {
    let mock = MockKeyring::new();

    let loaded = SecureSession::load_last_refresh_with_mock(&mock);
    assert!(loaded.is_ok());
    assert_eq!(loaded.unwrap(), None, "Missing timestamp should load as None");
}

#[test]
fn test_delete_clears_last_refresh() {
    let mock = MockKeyring::new();

    // Save a full session plus the timestamp
    let jwt = create_test_jwt("user", None, 3600);
    let session = SecureSession {
        jwt,
        user_id: "user".to_string(),
        email: None,
    };
    assert!(session.save_with_mock(&mock).is_ok());
    assert!(SecureSession::save_last_refresh_with_mock(&mock, 1_700_000_000).is_ok());

    // Delete the session
    assert!(SecureSession::delete_with_mock(&mock).is_ok());

    // Timestamp must be gone too
    let loaded = SecureSession::load_last_refresh_with_mock(&mock);
    assert_eq!(
        loaded.unwrap(),
        None,
        "delete should also clear last_refresh"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib auth::secure_session 2>&1 | tail -20`
Expected: FAIL — `no function or associated item named 'save_last_refresh_with_mock' found`

- [ ] **Step 3: Add the constant and helpers**

In `src/auth/secure_session.rs`, add the key constant after line 23 (`const EMAIL_KEY: &str = "user_email";`):

```rust
const REFRESH_KEY: &str = "last_refresh";
```

Add the production helpers inside `impl SecureSession` (after the `jwt_token` method, around line 139):

```rust
    pub fn save_last_refresh(timestamp: i64) -> Result<()> {
        let store = keyring_store::default_store();
        Self::save_last_refresh_with_store(&store, timestamp)
    }

    fn save_last_refresh_with_store(store: &dyn KeyringStore, timestamp: i64) -> Result<()> {
        store.set_password(SERVICE_NAME, REFRESH_KEY, &timestamp.to_string())
    }

    pub fn load_last_refresh() -> Result<Option<i64>> {
        let store = keyring_store::default_store();
        Self::load_last_refresh_with_store(&store)
    }

    fn load_last_refresh_with_store(store: &dyn KeyringStore) -> Result<Option<i64>> {
        match store.get_password(SERVICE_NAME, REFRESH_KEY)? {
            Some(value) => Ok(value.parse::<i64>().ok()),
            None => Ok(None),
        }
    }
```

In `delete_with_store` (currently ends after deleting `EMAIL_KEY`, around line 132), add before the final `Ok(())`:

```rust
        // Delete last_refresh timestamp
        store.delete_password(SERVICE_NAME, REFRESH_KEY)?;
```

Add the test-only mock wrappers to the `#[cfg(test)] impl SecureSession` block at the bottom of the file (after `delete_with_mock`, around line 155):

```rust
    pub fn save_last_refresh_with_mock(mock: &MockKeyring, timestamp: i64) -> Result<()> {
        Self::save_last_refresh_with_store(mock, timestamp)
    }

    pub fn load_last_refresh_with_mock(mock: &MockKeyring) -> Result<Option<i64>> {
        Self::load_last_refresh_with_store(mock)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib auth::secure_session 2>&1 | tail -20`
Expected: PASS — the three new `test_last_refresh*` / `test_delete_clears_last_refresh` tests plus existing secure_session tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/auth/secure_session.rs src/auth/secure_session_test.rs
git commit -m "feat(auth): persist last_refresh timestamp in secure store"
```

---

### Task 3: Wire daily rotation into the refresh task

Have `set_session` stamp `last_refresh` on every token write (best-effort), and have the hourly task use `refresh_due`. This is integration wiring; its building blocks are unit-tested in Tasks 1–2, so verification is a full build + test suite + a manual log note.

**Files:**
- Modify: `src/auth/mod.rs` (imports near line 7–11; `set_session` at lines 33–39; refresh task at lines 66–101)

- [ ] **Step 1: Add imports**

In `src/auth/mod.rs`, add after line 7 (`use log::{debug, error, info};`):

```rust
use chrono::Utc;
```

And add after line 11 (`use self::secure_session::SecureSession;`):

```rust
use self::jwt::refresh_due;
```

- [ ] **Step 2: Stamp the timestamp in `set_session`**

Replace the existing `set_session` (lines 33–39):

```rust
    pub fn set_session(&self, jwt_token: String) -> Result<()> {
        let session = SecureSession::new(jwt_token)?;
        session.save()?;

        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }
```

with:

```rust
    pub fn set_session(&self, jwt_token: String) -> Result<()> {
        let session = SecureSession::new(jwt_token)?;
        session.save()?;

        // Best-effort: record when this token was obtained. A missing timestamp
        // simply makes the next hourly tick treat the token as refresh-due, which
        // is safe — so a write failure here must not fail login/refresh.
        if let Err(e) = SecureSession::save_last_refresh(Utc::now().timestamp()) {
            error!("Failed to record last_refresh timestamp: {e}");
        }

        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }
```

- [ ] **Step 3: Replace the refresh-task decision logic**

In the `start_refresh_task` loop, replace the `match session.jwt_token()` block (lines 67–100) with:

```rust
                    match session.jwt_token() {
                        Ok(jwt) => {
                            let last_refresh =
                                SecureSession::load_last_refresh().unwrap_or(None);

                            if refresh_due(&jwt, last_refresh, Utc::now().timestamp()) {
                                info!("JWT token refresh due");

                                match self.api.refresh_token(&session.jwt).await {
                                    Ok(new_token) => {
                                        if let Err(e) = self.set_session(new_token) {
                                            error!("Failed to save refreshed token: {e}");
                                        } else {
                                            info!("JWT token refreshed successfully");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to refresh JWT token: {e}");

                                        // Clear invalid session
                                        if let Err(e) = self.clear_session() {
                                            error!("Failed to clear invalid session: {e}");
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Invalid JWT token: {e}");

                            // Clear invalid session
                            if let Err(e) = self.clear_session() {
                                error!("Failed to clear invalid session: {e}");
                            }
                        }
                    }
```

- [ ] **Step 4: Build and run the full test suite**

Run: `cargo build 2>&1 | tail -20`
Expected: builds with no errors.

Run: `cargo test 2>&1 | tail -20`
Expected: PASS — full suite green, including the Task 1 and Task 2 tests.

- [ ] **Step 5: Commit**

```bash
git add src/auth/mod.rs
git commit -m "feat(auth): rotate JWT at least daily via refresh_due"
```

---

### Task 4: Pre-push verification

Run the project's CI-matching checks (from the repo memory checklist) before the work is considered done.

**Files:** none (verification only)

- [ ] **Step 1: Format check**

Run: `cargo fmt --check`
Expected: no output (clean). If it reports diffs, run `cargo fmt` and re-commit with `git commit -am "style: cargo fmt"`.

- [ ] **Step 2: Clippy (CI flags)**

Run: `cargo clippy --all-targets -- -D warnings -A dead_code 2>&1 | tail -20`
Expected: finishes with no errors.

- [ ] **Step 3: Full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests pass.

- [ ] **Step 4: Manual sanity check of the behavior (optional but recommended)**

After installing the new build and letting the daemon run, the log at
`~/Library/Caches/cook-sync/cook-sync.log` (macOS) should show `JWT token refresh due`
followed by `JWT token refreshed successfully` within ~24h of the last refresh, instead of
the previous 4-month silence. Confirm with:

Run: `grep -c "JWT token refreshed successfully" ~/Library/Caches/cook-sync/cook-sync.log`
Expected: increments by roughly one per day the daemon is running.

---

## Notes for the implementer

- `refresh_token` already exists on `CookApi` and returns the new token string; this plan does not change it.
- `set_session` is intentionally left using the real `default_store()` (not store-injected). Its composed pieces — `refresh_due` and the `last_refresh` storage — are unit-tested directly, so no store-injection refactor of `set_session` is in scope.
- The hourly check cadence is unchanged. On wake, tokio's default `MissedTickBehavior::Burst` fires the missed tick promptly, so rotation resumes within an hour of waking — negligible against the ~14-day token lifetime.
