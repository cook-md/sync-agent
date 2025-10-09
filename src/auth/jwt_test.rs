use crate::auth::jwt::{JwtToken, UserId};
use base64::{engine::general_purpose, Engine as _};
use serde_json::json;

/// Helper to create a JWT with specific claims
fn create_jwt_with_claims(claims: serde_json::Value) -> String {
    let header = general_purpose::STANDARD_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = general_purpose::STANDARD_NO_PAD.encode(claims.to_string());
    format!("{header}.{payload}.test_signature")
}

#[test]
fn test_jwt_from_string_valid() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "uid": "test_user",
        "email": "test@example.com",
        "exp": now + 3600,
        "iat": now
    });

    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string.clone());

    assert!(jwt.is_ok(), "Should parse valid JWT");
    let jwt = jwt.unwrap();
    assert_eq!(jwt.token, jwt_string);
    assert_eq!(jwt.user_id(), "test_user");
    assert_eq!(jwt.claims.email, Some("test@example.com".to_string()));
}

#[test]
fn test_jwt_from_string_invalid_format() {
    // Test with wrong number of parts
    let invalid_jwts = vec!["invalid", "only.two", "too.many.parts.here", ""];

    for invalid in invalid_jwts {
        let result = JwtToken::from_string(invalid.to_string());
        assert!(
            result.is_err(),
            "Should fail for invalid JWT format: {invalid}"
        );
    }
}

#[test]
fn test_jwt_from_string_invalid_base64() {
    let invalid_jwt = "header.!!!invalid_base64!!!.signature";
    let result = JwtToken::from_string(invalid_jwt.to_string());
    assert!(result.is_err(), "Should fail for invalid base64 in payload");
}

#[test]
fn test_jwt_from_string_invalid_json() {
    let header = general_purpose::STANDARD_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = general_purpose::STANDARD_NO_PAD.encode("not valid json");
    let jwt_string = format!("{header}.{payload}.signature");

    let result = JwtToken::from_string(jwt_string);
    assert!(result.is_err(), "Should fail for invalid JSON in payload");
}

#[test]
fn test_jwt_with_integer_user_id() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "uid": 12345,  // Integer user ID
        "exp": now + 3600,
        "iat": now
    });

    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string).unwrap();

    assert_eq!(jwt.user_id(), "12345");
}

#[test]
fn test_jwt_with_string_user_id() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "uid": "string_user_id",
        "exp": now + 3600,
        "iat": now
    });

    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string).unwrap();

    assert_eq!(jwt.user_id(), "string_user_id");
}

#[test]
fn test_jwt_is_expired() {
    let now = chrono::Utc::now().timestamp();

    // Test expired token
    let expired_claims = json!({
        "uid": "user",
        "exp": now - 3600,  // Expired 1 hour ago
        "iat": now - 7200
    });
    let expired_jwt = create_jwt_with_claims(expired_claims);
    let jwt = JwtToken::from_string(expired_jwt).unwrap();
    assert!(jwt.is_expired(), "Token should be expired");

    // Test valid token
    let valid_claims = json!({
        "uid": "user",
        "exp": now + 3600,  // Expires in 1 hour
        "iat": now
    });
    let valid_jwt = create_jwt_with_claims(valid_claims);
    let jwt = JwtToken::from_string(valid_jwt).unwrap();
    assert!(!jwt.is_expired(), "Token should not be expired");
}

#[test]
fn test_jwt_expires_in() {
    let now = chrono::Utc::now().timestamp();

    // Token that expires in 2 hours
    let claims = json!({
        "uid": "user",
        "exp": now + 7200,
        "iat": now
    });
    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string).unwrap();

    let expires_in = jwt.expires_in();
    // Should be approximately 2 hours (allowing for small time differences during test execution)
    assert!(expires_in.num_seconds() > 7190 && expires_in.num_seconds() <= 7200);
}

#[test]
fn test_jwt_should_refresh() {
    let now = chrono::Utc::now().timestamp();

    // Token expiring in 30 minutes - should refresh
    let soon_expiring = json!({
        "uid": "user",
        "exp": now + 1800,  // 30 minutes
        "iat": now
    });
    let jwt = JwtToken::from_string(create_jwt_with_claims(soon_expiring)).unwrap();
    assert!(
        jwt.should_refresh(),
        "Token expiring in 30 minutes should be refreshed"
    );

    // Token expiring in 2 hours - should not refresh yet
    let later_expiring = json!({
        "uid": "user",
        "exp": now + 7200,  // 2 hours
        "iat": now
    });
    let jwt = JwtToken::from_string(create_jwt_with_claims(later_expiring)).unwrap();
    assert!(
        !jwt.should_refresh(),
        "Token expiring in 2 hours should not be refreshed yet"
    );
}

#[test]
fn test_jwt_without_email() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "uid": "user_without_email",
        "exp": now + 3600,
        "iat": now
        // No email field
    });

    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string).unwrap();

    assert_eq!(jwt.user_id(), "user_without_email");
    assert_eq!(jwt.claims.email, None);
}

#[test]
fn test_jwt_without_iat() {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "uid": "user",
        "exp": now + 3600
        // No iat field - should default to 0
    });

    let jwt_string = create_jwt_with_claims(claims);
    let jwt = JwtToken::from_string(jwt_string).unwrap();

    assert_eq!(jwt.claims.iat, 0);
}

#[test]
fn test_user_id_as_string() {
    // Test integer user ID
    let int_id = UserId::Integer(42);
    assert_eq!(int_id.as_string(), "42");

    // Test string user ID
    let str_id = UserId::String("user_123".to_string());
    assert_eq!(str_id.as_string(), "user_123");
}
