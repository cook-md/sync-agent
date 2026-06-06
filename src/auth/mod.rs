pub mod device_flow;
pub mod jwt;
pub mod secure_session;

use crate::api::CookApi;
use crate::config::AppPaths;
use crate::error::{Result, SyncError};
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

use self::device_flow::{client_name, poll_for_token, request_device_code};
use self::secure_session::SecureSession;

pub struct AuthManager {
    api: Arc<CookApi>,
    session: Arc<Mutex<Option<SecureSession>>>,
}

impl AuthManager {
    pub fn new(_paths: Arc<AppPaths>, api: Arc<CookApi>) -> Result<Self> {
        // Load session from keyring
        let session = SecureSession::load()?;

        Ok(Self {
            api,
            session: Arc::new(Mutex::new(session)),
        })
    }

    pub fn get_session(&self) -> Option<SecureSession> {
        self.session.lock().unwrap().clone()
    }

    pub fn set_session(&self, jwt_token: String) -> Result<()> {
        let session = SecureSession::new(jwt_token)?;
        session.save()?;

        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }

    pub fn clear_session(&self) -> Result<()> {
        SecureSession::delete()?;
        *self.session.lock().unwrap() = None;
        Ok(())
    }

    pub fn is_authenticated(&self) -> bool {
        self.get_session().is_some()
    }

    pub fn logout(&self) -> Result<()> {
        self.clear_session()
    }

    pub async fn start_token_refresh(self: &Arc<Self>) {
        self.clone().start_refresh_task();
    }

    pub fn start_refresh_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut refresh_interval = interval(Duration::from_secs(3600)); // Check every hour

            loop {
                refresh_interval.tick().await;

                if let Some(session) = self.get_session() {
                    match session.jwt_token() {
                        Ok(jwt) if jwt.should_refresh() => {
                            info!("JWT token needs refresh");

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
                        Ok(_) => {
                            // Token still valid
                        }
                        Err(e) => {
                            error!("Invalid JWT token: {e}");

                            // Clear invalid session
                            if let Err(e) = self.clear_session() {
                                error!("Failed to clear invalid session: {e}");
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn browser_login(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        fn show_login_url_dialog(url: &str) {
            // Best-effort: copy the URL to the clipboard via xclip / wl-copy so the
            // user can paste it without retyping. Both are common but optional.
            use std::io::Write;
            use std::process::Stdio;
            let mut clipboard_copied = false;
            for (program, args) in [
                ("wl-copy", &[][..]),
                ("xclip", &["-selection", "clipboard"][..]),
            ] {
                let mut cmd =
                    crate::platform::linux::desktop_integration::clean_appimage_env(program);
                cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::null());
                if let Ok(mut child) = cmd.spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(url.as_bytes());
                    }
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            clipboard_copied = true;
                            break;
                        }
                    }
                }
            }

            let body = if clipboard_copied {
                format!("Couldn't open a browser automatically.\n\nThe sign-in URL has been copied to your clipboard — paste it into any browser to continue.\n\n{url}")
            } else {
                format!("Couldn't open a browser automatically.\n\nOpen this URL in any browser to continue sign-in:\n\n{url}")
            };

            let _ = crate::platform::linux::desktop_integration::clean_appimage_env("zenity")
                .args([
                    "--info",
                    "--title=Cook Sync — Sign In",
                    "--no-markup",
                    "--text",
                    &body,
                ])
                .status();
        }

        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        use tokio::time::timeout;

        // Start local HTTP server on random port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        // Generate state for CSRF protection
        let state = uuid::Uuid::new_v4().to_string();

        // Construct callback URL
        let callback_url = format!("http://127.0.0.1:{port}/auth/callback");
        let encoded_callback = urlencoding::encode(&callback_url);

        // Get base URL from API endpoint
        let api_endpoint = self.api.base_url();
        let base_url = api_endpoint.strip_suffix("/api").unwrap_or(api_endpoint);

        // Open browser to desktop authentication page
        let login_url =
            format!("{base_url}/auth/desktops?callback={encoded_callback}&state={state}");

        info!("Opening browser for authentication: {login_url}");

        // Print the URL up front so terminal users can copy it even if the
        // automatic browser open succeeds — and especially if it doesn't.
        println!("\nIf your browser doesn't open automatically, visit:\n");
        println!("    {login_url}\n");

        if let Err(e) = open::that(&login_url) {
            // Don't abort: some Linux setups (e.g. Pop_OS/Cosmic) have a broken
            // xdg-open that fails even when a usable browser is installed.
            // Keep the local listener alive so the user can paste the URL manually.
            error!("Failed to open browser automatically: {e}");

            // Notification — works on all platforms, but body may be truncated.
            let _ = crate::notifications::show_notification(
                "Cook Sync — couldn't open browser",
                "Sign-in URL has been shown on screen. Open it in any browser to continue.",
            );

            // On Linux the daemon has no visible stdout when launched from the
            // tray, so show a selectable dialog with the URL as a GUI fallback.
            #[cfg(target_os = "linux")]
            show_login_url_dialog(&login_url);
        }

        // Wait for callback (with timeout)
        let result = timeout(Duration::from_secs(300), async {
            let (mut socket, _) = listener.accept().await?;

            // Read initial request
            let mut buffer = [0; 4096];
            let n = socket.read(&mut buffer).await?;
            let request = String::from_utf8_lossy(&buffer[..n]);

            // Check if this is a CORS preflight request
            if request.starts_with("OPTIONS ") {
                debug!("Handling CORS preflight request");
                // Send CORS headers for preflight
                let cors_response = "HTTP/1.1 200 OK\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                    Access-Control-Allow-Headers: x-csrf-token, x-turbo-request-id\r\n\
                    Access-Control-Max-Age: 86400\r\n\
                    Content-Length: 0\r\n\r\n";
                socket.write_all(cors_response.as_bytes()).await?;

                // The browser might reuse the same connection or open a new one
                // Try to read another request on the same connection first
                let mut buffer = [0; 4096];
                match tokio::time::timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                    Ok(Ok(n)) if n > 0 => {
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        self.handle_callback_request(&mut socket, &request, &state)
                            .await
                    }
                    _ => {
                        // If no data on same connection, accept a new connection
                        let (mut new_socket, _) = listener.accept().await?;
                        let mut buffer = [0; 4096];
                        let n = new_socket.read(&mut buffer).await?;
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        self.handle_callback_request(&mut new_socket, &request, &state)
                            .await
                    }
                }
            } else {
                // Regular GET request
                self.handle_callback_request(&mut socket, &request, &state)
                    .await
            }
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SyncError::Other("Authentication timeout".to_string())),
        }
    }

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

        let jwt_result = poll_for_token(
            &client,
            &base_url,
            &dc.device_code,
            interval,
            expires_at,
            cancel.clone(),
        )
        .await;

        cancel.cancel();
        dot_handle.abort();
        println!();

        let jwt = jwt_result?;
        self.set_session(jwt)?;

        if let Some(session) = self.get_session() {
            println!("Logged in as {}", session.email.unwrap_or(session.user_id));
        }

        Ok(())
    }

    async fn handle_callback_request(
        &self,
        socket: &mut tokio::net::TcpStream,
        request: &str,
        expected_state: &str,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        debug!("Parsing request for token extraction");
        if let Some(token) = Self::extract_token_from_request(request, expected_state) {
            debug!("Token extracted successfully");

            // Send success response with styled HTML - show "All Done" directly
            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\n\r\n\
<!DOCTYPE html>
<html>
<head>
  <title>Authentication Complete - Cook.md</title>
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      margin: 0;
      background-color: #f5f5f5;
    }
    .container {
      text-align: center;
      background: white;
      padding: 2rem 3rem;
      border-radius: 8px;
      box-shadow: 0 2px 10px rgba(0,0,0,0.1);
      max-width: 400px;
    }
    .success-icon {
      width: 64px;
      height: 64px;
      margin: 0 auto 1.5rem;
      background-color: #4CAF50;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 32px;
      color: white;
    }
    h1 {
      color: #333;
      font-size: 1.5rem;
      margin: 0 0 0.5rem;
    }
    p {
      color: #666;
      margin: 0 0 1rem;
      line-height: 1.5;
    }
    .note {
      color: #999;
      font-size: 0.9rem;
    }
  </style>
</head>
<body>
  <div class=\"container\">
    <div class=\"success-icon\">✓</div>

    <h1>All Done!</h1>

    <p>
      Authentication complete. You can now safely close this browser tab.
    </p>

    <p class=\"note\">
      Return to Cook Sync to continue.
    </p>
  </div>

  <script>
    // Try to close the window automatically if it was opened as a popup
    setTimeout(function() {
      if (window.opener) {
        window.close();
      }
    }, 2000);

    // Try to blur the window to hint user to close it
    window.blur();
  </script>
</body>
</html>";
            socket.write_all(response.as_bytes()).await?;

            // Save the session
            self.set_session(token)?;
            Ok(())
        } else {
            debug!("Failed to extract token from request");

            // Send error response with styled HTML
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\n\r\n\
<!DOCTYPE html>
<html>
<head>
  <title>Authentication Failed - Cook.md</title>
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      margin: 0;
      background-color: #f5f5f5;
    }
    .container {
      text-align: center;
      background: white;
      padding: 2rem 3rem;
      border-radius: 8px;
      box-shadow: 0 2px 10px rgba(0,0,0,0.1);
      max-width: 400px;
    }
    .error-icon {
      width: 64px;
      height: 64px;
      margin: 0 auto 1.5rem;
      background-color: #f44336;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 32px;
      color: white;
    }
    h1 {
      color: #333;
      font-size: 1.5rem;
      margin: 0 0 0.5rem;
    }
    p {
      color: #666;
      margin: 0 0 1.5rem;
      line-height: 1.5;
    }
    .button {
      background-color: #f44336;
      color: white;
      border: none;
      padding: 0.75rem 2rem;
      font-size: 1rem;
      border-radius: 4px;
      cursor: pointer;
      transition: background-color 0.3s;
    }
    .button:hover {
      background-color: #d32f2f;
    }
  </style>
</head>
<body>
  <div class=\"container\">
    <div class=\"error-icon\">✕</div>
    
    <h1>Authentication Failed</h1>
    
    <p>
      The authentication process could not be completed. 
      Please close this window and try again.
    </p>
    
    <button class=\"button\" onclick=\"window.close()\">
      Close Window
    </button>
  </div>
</body>
</html>";
            socket.write_all(response.as_bytes()).await?;
            Err(SyncError::AuthenticationRequired)
        }
    }

    fn extract_token_from_request(request: &str, expected_state: &str) -> Option<String> {
        // Parse GET request for token and state parameters
        debug!("Full request:\n{request}");
        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];
        if !first_line.starts_with("GET ") {
            return None;
        }

        // Extract path and query string
        let parts: Vec<&str> = first_line.split(' ').collect();
        if parts.len() < 2 {
            return None;
        }

        let path_and_query = parts[1];
        debug!("Path and query: {path_and_query}");
        if let Some(query_start) = path_and_query.find('?') {
            let query = &path_and_query[query_start + 1..];

            let mut token = None;
            let mut state = None;

            // Parse query parameters
            for param in query.split('&') {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[..eq_pos];
                    let value = &param[eq_pos + 1..];

                    match key {
                        "token" => token = Some(urlencoding::decode(value).ok()?.into_owned()),
                        "state" => state = Some(urlencoding::decode(value).ok()?.into_owned()),
                        _ => {}
                    }
                }
            }

            // Verify state matches
            if state.as_deref() == Some(expected_state) {
                token
            } else {
                None
            }
        } else {
            None
        }
    }
}
