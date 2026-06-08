use super::{client_name, interpret_token_response, server_host_label, PollOutcome};

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

#[test]
fn server_host_label_is_known_value() {
    let label = server_host_label();
    assert!(label == "docker" || label == "server");
}
