use super::keyring_store::MockKeyring;
use super::SecureSession;

/// Helper function to create a valid JWT token for testing
fn create_test_jwt(user_id: &str, email: Option<&str>, expires_in_seconds: i64) -> String {
    use base64::{engine::general_purpose, Engine as _};
    use serde_json::json;

    let now = chrono::Utc::now().timestamp();
    let header = general_purpose::STANDARD_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let claims = json!({
        "uid": user_id,
        "email": email,
        "exp": now + expires_in_seconds,
        "iat": now
    });
    let payload = general_purpose::STANDARD_NO_PAD.encode(claims.to_string());
    format!("{header}.{payload}.test_signature")
}

#[test]
fn test_secure_session_new() {
    let jwt = create_test_jwt("test_user", Some("test@example.com"), 3600);
    let session = SecureSession::new(jwt.clone());

    assert!(session.is_ok(), "Should create session from valid JWT");
    let session = session.unwrap();
    assert_eq!(session.jwt, jwt);
    assert_eq!(session.user_id, "test_user");
    assert_eq!(session.email, Some("test@example.com".to_string()));
}

#[test]
fn test_secure_session_new_with_invalid_jwt() {
    let invalid_jwt = "invalid.jwt.token";
    let session = SecureSession::new(invalid_jwt.to_string());

    assert!(session.is_err(), "Should fail with invalid JWT");
}

#[test]
fn test_secure_session_save_and_load() {
    let mock = MockKeyring::new();

    let jwt = create_test_jwt("test_user", Some("test@example.com"), 3600);
    let session = SecureSession {
        jwt: jwt.clone(),
        user_id: "test_user".to_string(),
        email: Some("test@example.com".to_string()),
    };

    // Save session
    assert!(
        session.save_with_mock(&mock).is_ok(),
        "Failed to save session to mock keyring"
    );

    // Load session
    let loaded = SecureSession::load_with_mock(&mock);
    assert!(loaded.is_ok(), "Failed to load session from mock keyring");

    let loaded_session = loaded.unwrap();
    assert!(
        loaded_session.is_some(),
        "Session should exist after saving"
    );

    let session_data = loaded_session.unwrap();
    assert_eq!(session_data.jwt, jwt);
    assert_eq!(session_data.user_id, "test_user");
    assert_eq!(session_data.email, Some("test@example.com".to_string()));
}

#[test]
fn test_secure_session_handles_missing_session() {
    let mock = MockKeyring::new();

    // Try to load non-existent session
    let result = SecureSession::load_with_mock(&mock);
    assert!(
        result.is_ok(),
        "Loading non-existent session should not error"
    );
    assert!(
        result.unwrap().is_none(),
        "Non-existent session should return None"
    );
}

#[test]
fn test_secure_session_auto_cleanup_expired_token() {
    let mock = MockKeyring::new();

    // Create an expired JWT (expired 1 hour ago)
    let expired_jwt = create_test_jwt("expired_user", Some("expired@example.com"), -3600);

    let session = SecureSession {
        jwt: expired_jwt,
        user_id: "expired_user".to_string(),
        email: Some("expired@example.com".to_string()),
    };

    // Save the expired session
    assert!(
        session.save_with_mock(&mock).is_ok(),
        "Should save even expired session"
    );

    // Try to load - should return None due to expiration
    let loaded = SecureSession::load_with_mock(&mock);
    assert!(loaded.is_ok(), "Loading should not error");
    assert!(
        loaded.unwrap().is_none(),
        "Should return None for expired session"
    );

    // Verify the expired session was cleaned up
    let second_load = SecureSession::load_with_mock(&mock);
    assert!(second_load.is_ok());
    assert!(
        second_load.unwrap().is_none(),
        "Session should remain deleted"
    );
}

#[test]
fn test_secure_session_delete() {
    let mock = MockKeyring::new();

    // Create and save a session
    let jwt = create_test_jwt("delete_test_user", None, 3600);
    let session = SecureSession {
        jwt,
        user_id: "delete_test_user".to_string(),
        email: None,
    };

    assert!(session.save_with_mock(&mock).is_ok());

    // Verify it exists
    let loaded = SecureSession::load_with_mock(&mock);
    assert!(
        loaded.unwrap().is_some(),
        "Session should exist before deletion"
    );

    // Delete it
    assert!(
        SecureSession::delete_with_mock(&mock).is_ok(),
        "Deletion should succeed"
    );

    // Verify it's gone
    let after_delete = SecureSession::load_with_mock(&mock);
    assert!(
        after_delete.unwrap().is_none(),
        "Session should not exist after deletion"
    );

    // Delete again should not error
    assert!(
        SecureSession::delete_with_mock(&mock).is_ok(),
        "Deleting non-existent session should succeed"
    );
}

#[test]
fn test_secure_session_jwt_token_method() {
    let jwt = create_test_jwt("method_test_user", Some("method@test.com"), 3600);
    let session = SecureSession {
        jwt: jwt.clone(),
        user_id: "method_test_user".to_string(),
        email: Some("method@test.com".to_string()),
    };

    let jwt_token = session.jwt_token();
    assert!(jwt_token.is_ok(), "Should extract JWT token");

    let token = jwt_token.unwrap();
    assert_eq!(token.user_id(), "method_test_user");
    assert_eq!(token.claims.email, Some("method@test.com".to_string()));
    assert!(!token.is_expired(), "Token should not be expired");
}

#[test]
fn test_secure_session_without_email() {
    let mock = MockKeyring::new();

    let jwt = create_test_jwt("no_email_user", None, 3600);
    let session = SecureSession {
        jwt: jwt.clone(),
        user_id: "no_email_user".to_string(),
        email: None,
    };

    // Save and load
    assert!(session.save_with_mock(&mock).is_ok());

    let loaded = SecureSession::load_with_mock(&mock);
    assert!(loaded.is_ok());

    let loaded_session = loaded.unwrap().unwrap();
    assert_eq!(loaded_session.user_id, "no_email_user");
    assert_eq!(loaded_session.email, None);
}

#[test]
fn test_secure_session_with_integer_user_id() {
    let jwt = create_test_jwt("12345", Some("int_user@test.com"), 3600);
    let session = SecureSession::new(jwt);

    assert!(session.is_ok());
    let session = session.unwrap();
    assert_eq!(session.user_id, "12345");
}
