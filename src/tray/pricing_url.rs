//! The pricing link the tray opens. Carries the door (`from=sync_agent`) so
//! cook.md can attribute the visit, and the account email so its sign-in
//! strip is prefilled — the agent's token is not a web session.

/// Build the pricing URL for the tray's Subscribe item.
pub fn pricing_url(email: Option<&str>) -> String {
    let base = "https://cook.md/pricing?from=sync_agent";
    match email.map(str::trim).filter(|e| !e.is_empty()) {
        Some(email) => format!("{base}&email={}", urlencoding::encode(email)),
        None => base.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::pricing_url;

    #[test]
    fn carries_the_door_and_the_email() {
        assert_eq!(
            pricing_url(Some("a+b@example.com")),
            "https://cook.md/pricing?from=sync_agent&email=a%2Bb%40example.com"
        );
    }

    #[test]
    fn omits_the_email_when_there_is_none() {
        assert_eq!(pricing_url(None), "https://cook.md/pricing?from=sync_agent");
        assert_eq!(
            pricing_url(Some("  ")),
            "https://cook.md/pricing?from=sync_agent"
        );
    }
}
