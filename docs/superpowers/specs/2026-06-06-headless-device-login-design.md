# Headless device-code login

**Issue:** [#96 — Headless login for use on a server](https://github.com/cook-md/sync-agent/issues/96)
**Date:** 2026-06-06
**Status:** Approved design

## Problem

`cook-sync login` only supports a browser-based loopback flow: it opens a local
HTTP listener on `127.0.0.1`, opens the user's browser to the cook.md desktop
auth page, and waits for a redirect carrying the JWT. On a headless server or
inside a Docker container there is no browser and no display, so there is no way
to authenticate. The user in #96 wants to sync a recipes folder on an Ubuntu
server but cannot log in there.

## Solution overview

Add OAuth 2.0 **device authorization grant** (RFC 8628) login, ported from the
working implementation in cookcli (PR cook-md/CookCLI#347). The cook.md server
already exposes the endpoints this needs:

- `POST /oauth/device/code` — returns a device code, user code, verification URI,
  poll interval, and expiry.
- `POST /oauth/device/token` — polled until the user approves; returns the JWT.

(Confirmed in cook.md `web/config/routes.rb`, the `namespace :oauth` block. These
routes live at the site root, **not** under `/api`.)

This is purely client-side work; no server changes are required.

### Flow selection

`cook-sync login` gains a `--headless` flag. The command chooses a flow:

| Condition | Flow |
|---|---|
| `--headless` passed | device flow (always) |
| no flag, environment looks headless | device flow (auto) |
| otherwise | existing browser/loopback flow (unchanged) |

"Looks headless" is true when:

- `/.dockerenv` exists, **or**
- on Linux, neither `DISPLAY` nor `WAYLAND_DISPLAY` is set.

On macOS and Windows there is always a GUI session, so auto-detection never
triggers there — only the explicit `--headless` flag selects the device flow.

### Convergence

Both flows end identically: the obtained JWT is handed to the existing
`AuthManager::set_session()`, which persists it to the platform store and records
the refresh timestamp. On Linux/Docker that store is already file-based
(`~/.config/cook-sync/session-store.json`, mode 0600), so persistence works on a
headless server with no Secret Service / keyring dependency.

### Why keep both flows

cookcli replaced its browser flow with device flow outright. sync-agent is
primarily a desktop/tray app where one-click browser login is the better UX, so
we keep both and default intelligently. Desktop users see no change; server and
Docker users get a working path. No regression to the existing flow.

## Components

### New: `src/auth/device_flow.rs`

A close port of cookcli's `device_flow.rs`, adapted to sync-agent's `SyncError`:

- Response/request structs: `DeviceCodeResponse`, `DeviceCodeRequest`,
  `TokenRequest`, `TokenSuccess`, `TokenError` (serde).
- `request_device_code(client, base_url, client_name) -> Result<DeviceCodeResponse>`
  — POSTs to `{base_url}/oauth/device/code`.
- `poll_for_token(client, base_url, device_code, interval, expires_at, cancel) -> Result<String>`
  — RFC 8628 polling state machine against `{base_url}/oauth/device/token`:
  - `authorization_pending` → keep waiting at the current interval
  - `slow_down` → increase interval by 5s
  - `access_denied` → terminal error
  - `expired_token` (or local deadline reached) → terminal error
  - HTTP success → return the JWT (`access_token`)
- `client_name() -> String` — e.g. `"Cook Sync 0.6.x (linux/docker)"`, so the
  cook.md approval screen identifies the device. Includes OS and a best-effort
  `docker`/`server` label (`/.dockerenv` check).

Errors are surfaced through the existing `SyncError` type (a new variant or
mapped messages), not a separate public error enum, to match the rest of the
auth module.

### Changed: `src/auth/mod.rs` — `AuthManager::device_login()`

New sibling to `browser_login()`:

1. Compute base URL by stripping the `/api` suffix from the API endpoint (reuse
   the same logic `browser_login()` already uses).
2. `request_device_code(...)`.
3. Print the verification URI and the user code to the terminal. Offer to open
   the browser automatically (Enter to open via `open::that`, Ctrl-C to abort) —
   harmless on a desktop, ignorable on a server where the user reads the code.
4. Poll with a simple progress indicator, respecting `interval`, the `expires_in`
   deadline, `slow_down`, and Ctrl-C cancellation (via `tokio_util`
   `CancellationToken` + `tokio::signal::ctrl_c`, both already available).
5. On success call `set_session(jwt)` and print `Logged in as <email>`.

### Changed: `src/main.rs`

- `Commands::Login` gains `headless: bool` (`#[arg(long)]`).
- `fn prefer_device_flow(headless: bool) -> bool` — holds the detection rule
  (flag OR docker OR Linux-no-display). Pure and unit-testable.
- `login()` calls `prefer_device_flow(...)` and dispatches to
  `auth.device_login()` or `auth.browser_login()`.

The tray/welcome-screen login path (`login_requested`) is GUI-only and continues
to use `browser_login()` unchanged.

## Data flow

```
cook-sync login [--headless]
  └─ prefer_device_flow()? ── no ──> browser_login()  (existing, unchanged)
                            └─ yes ─> device_login()
                                        ├─ request_device_code()  POST /oauth/device/code
                                        ├─ print user_code + verification_uri
                                        ├─ (optional) open::that(verification_uri_complete)
                                        ├─ poll_for_token()        POST /oauth/device/token  (loop)
                                        └─ set_session(jwt)  ──>  platform store + last_refresh
```

## Error handling

| Situation | Behaviour |
|---|---|
| device-code request HTTP failure | clear error message, no session saved |
| `access_denied` | "Authorization denied." |
| `expired_token` / local deadline | "Code expired — run `cook-sync login` again." |
| Ctrl-C during wait | clean "Cancelled.", no session saved |
| network blip while polling | propagate as `SyncError`; no partial state |

No partial or corrupt session is ever persisted — `set_session` is only called
once a valid JWT is in hand.

## Testing

Dev-dependencies currently include only `tempfile` (no HTTP mocking library). To
stay dependency-free, `poll_for_token`'s HTTP call is made injectable behind a
small trait (or `Fn` seam) so the state machine is unit-tested without a network:

- `authorization_pending` → loops, does not return
- `slow_down` → interval increases by 5s
- `access_denied` → returns the access-denied error
- `expired_token` / past-deadline → returns the expired error
- success body → returns the JWT

Plus unit tests for `prefer_device_flow()` covering the matrix: `--headless`
override true/false; `/.dockerenv` present; Linux with/without `DISPLAY` and
`WAYLAND_DISPLAY`; non-Linux platforms. This mirrors the repo's existing
mock-store testing style in `secure_session_test.rs`.

## Out of scope (follow-up)

`cook-sync start` (the daemon) hard-fails when the system tray cannot initialize
(`src/daemon.rs:244` returns the error). On a truly headless server with no
display, the daemon therefore will not start even after a successful headless
login. Making the daemon run without a tray (e.g. a `--no-tray` mode) is a
separate change and is **not** part of this spec. It should be filed as a
follow-up issue so that headless login plus headless daemon together fully close
out the server use case in #96.

## Dependencies

All required crates are already present: `tokio` (full), `tokio-util`
(CancellationToken), `reqwest`, `serde`, `thiserror`, `open`, `urlencoding`,
`log`. No new dependencies.
