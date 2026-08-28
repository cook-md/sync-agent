/// Translates raw error strings from `cooklang-sync-client` into short,
/// human-readable messages suitable for the tray menu and status output.
///
/// The sync client reports errors as either Rust debug dumps (e.g.
/// `ReqwestError(reqwest::Error { kind: Request, url: "...", source: ... })`)
/// or thiserror Display strings. Neither is fit for end-user UI, so we map
/// known patterns to friendly text and truncate anything unrecognized.
/// The raw message is always logged before this translation is applied.
const MAX_DISPLAY_LEN: usize = 80;

pub fn humanize_error(raw: &str) -> String {
    let lower = raw.to_lowercase();

    if lower.contains("unauthorized") {
        return "Session expired — please log in again".to_string();
    }

    // Exact-match the known PaymentRequired Display/Debug text rather than a
    // "payment"/"paid plan" substring: a substring check can misfire on an
    // unrelated error whose message happens to embed a user's folder path
    // (e.g. ".../payment-receipts/recipes"), mislabeling it as a billing
    // issue.
    if lower == "sync requires a paid plan" || lower == "paymentrequired" {
        return "Sync needs a Cook Basic or Pro plan — your files are untouched.".to_string();
    }

    if lower.contains("timedout") || lower.contains("timed out") || lower.contains("timeout") {
        return "Network timeout — will retry".to_string();
    }

    if lower.contains("dns") || lower.contains("lookup") {
        return "Can't reach server — check your connection".to_string();
    }

    if lower.contains("reqwest")
        || lower.contains("hyper")
        || lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("incompletemessage")
        || lower.contains("sendrequest")
    {
        return "Connection problem — will retry".to_string();
    }

    if lower.contains("batch download") {
        return "Download failed — will retry".to_string();
    }

    if lower.contains("database") || lower.contains("dbqueryerror") {
        return "Local database error — see logs".to_string();
    }

    if lower.contains("io error") || lower.contains("ioerror") {
        return "File access error — see logs".to_string();
    }

    truncate_chars(raw, MAX_DISPLAY_LEN)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{}…", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reqwest_debug_dump_becomes_connection_problem() {
        let raw = r#"ReqwestError(reqwest::Error { kind: Request, url: "https://cook.md/api/metadata/poll?seconds=70&uuid=e9850dc7-1234", source: hyper_util::client::legacy::Error(SendRequest, hyper::Error(IncompleteMessage)) })"#;
        assert_eq!(humanize_error(raw), "Connection problem — will retry");
    }

    #[test]
    fn unauthorized_prompts_relogin() {
        assert_eq!(
            humanize_error("Unauthorized token"),
            "Session expired — please log in again"
        );
        assert_eq!(
            humanize_error("Unauthorized"),
            "Session expired — please log in again"
        );
    }

    #[test]
    fn payment_required_prompts_upgrade() {
        assert_eq!(
            humanize_error("Sync requires a paid plan"),
            "Sync needs a Cook Basic or Pro plan — your files are untouched."
        );
        assert_eq!(
            humanize_error("PaymentRequired"),
            "Sync needs a Cook Basic or Pro plan — your files are untouched."
        );
    }

    #[test]
    fn payment_substring_in_unrelated_path_does_not_misfire() {
        // A folder path that happens to contain "payment" must not be
        // mislabeled as a billing issue — only the exact PaymentRequired
        // Display/Debug text should match.
        let raw =
            "IoError { path: \"/Users/x/payment-receipts/recipes\", source: Os { code: 13 } }";
        assert_eq!(humanize_error(raw), "File access error — see logs");
    }

    #[test]
    fn timeout_detected_inside_debug_dump() {
        let raw = r#"ReqwestError(reqwest::Error { kind: Request, source: TimedOut })"#;
        assert_eq!(humanize_error(raw), "Network timeout — will retry");
    }

    #[test]
    fn dns_failure_reported_as_unreachable() {
        let raw = "Reqwest error error sending request: dns error: failed to lookup address";
        assert_eq!(
            humanize_error(raw),
            "Can't reach server — check your connection"
        );
    }

    #[test]
    fn database_error_is_summarized() {
        let raw = "Database query error database is locked";
        assert_eq!(humanize_error(raw), "Local database error — see logs");
    }

    #[test]
    fn io_error_is_summarized() {
        let raw = "IoError { path: \"/Users/x/recipes\", source: Os { code: 13 } }";
        assert_eq!(humanize_error(raw), "File access error — see logs");
    }

    #[test]
    fn short_unknown_message_passes_through() {
        assert_eq!(humanize_error("Sync failed"), "Sync failed");
    }

    #[test]
    fn long_unknown_message_is_truncated() {
        let raw = "x".repeat(200);
        let result = humanize_error(&raw);
        assert!(result.chars().count() <= MAX_DISPLAY_LEN + 1);
        assert!(result.ends_with('…'));
    }
}
