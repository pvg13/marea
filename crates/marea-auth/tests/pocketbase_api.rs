//! Integration tests for the PocketBase HTTP surface against a mock server.

use marea_auth::{PocketBase, PocketBaseError};
use wiremock::matchers::{body_json_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const DOMAIN: &[u8] = b"test.psk.v1|";

fn auth_ok_body(user_id: &str, email: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "token": token,
        "record": { "id": user_id, "email": email, "collectionName": "users", "verified": true }
    })
}

#[tokio::test]
async fn login_returns_session_with_derived_psk() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/auth-with-password"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(auth_ok_body("user123", "a@b.es", "tok-1")),
        )
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let session = pb.login("a@b.es", "hunter2").await.unwrap();
    assert_eq!(session.user_id, "user123");
    assert_eq!(session.email, "a@b.es");
    assert_eq!(session.token, "tok-1");
    // PSK derived locally, never sent: matches the free-function derivation.
    assert_eq!(
        session.psk,
        marea_auth::derive_psk(DOMAIN, "user123", "hunter2").unwrap()
    );
}

#[tokio::test]
async fn login_failure_maps_to_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/auth-with-password"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":400,"message":"Failed to authenticate.","data":{}}"#),
        )
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let err = pb.login("a@b.es", "wrong").await.unwrap_err();
    let PocketBaseError::Auth { message, fields } = err else {
        panic!("expected Auth, got {err:?}");
    };
    assert!(fields.is_empty());
    assert_eq!(message, "Failed to authenticate.");
}

#[tokio::test]
async fn register_creates_then_logs_in() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/records"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "newuser1234567",
            "email": "n@b.es"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/auth-with-password"))
        .respond_with(ResponseTemplate::new(200).set_body_json(auth_ok_body(
            "newuser1234567",
            "n@b.es",
            "tok-new",
        )))
        .expect(1)
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let session = pb.register("n@b.es", "hunter22").await.unwrap();
    assert_eq!(session.user_id, "newuser1234567");
    assert_eq!(session.token, "tok-new");
}

#[tokio::test]
async fn register_duplicate_email_surfaces_field_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/records"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"code":400,"message":"Failed to create record.","data":{"email":{"code":"validation_not_unique","message":"Value must be unique."}}}"#,
        ))
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let err = pb.register("dup@b.es", "hunter22").await.unwrap_err();
    let PocketBaseError::Auth { fields, .. } = &err else {
        panic!("expected Auth, got {err:?}");
    };
    assert_eq!(fields[0].field, "email");
    assert_eq!(fields[0].code, "validation_not_unique");
    assert!(err.user_message().contains("Email"));
}

#[tokio::test]
async fn refresh_preserves_psk_and_sends_raw_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/users/auth-refresh"))
        // Raw token in Authorization — matches what the production apps send.
        .and(header("authorization", "tok-old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(auth_ok_body(
            "user123",
            "a@b.es",
            "tok-refreshed",
        )))
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let old = marea_auth::AuthSession {
        user_id: "user123".into(),
        email: "a@b.es".into(),
        token: "tok-old".into(),
        psk: "cafe".repeat(16),
    };
    let new = pb.refresh(&old).await.unwrap();
    assert_eq!(new.token, "tok-refreshed");
    assert_eq!(new.psk, old.psk, "refresh must preserve the PSK");
}

#[tokio::test]
async fn ensure_fresh_skips_refresh_for_fresh_token() {
    // Fresh JWT (exp year 2100) → no HTTP call must happen. The mock server
    // has no mounts, so any request would 404 and fail the test.
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_b64 = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(r#"{"exp":4102444800}"#);
    let token = format!("{header_b64}.{payload}.sig");

    let server = MockServer::start().await;
    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let session = marea_auth::AuthSession {
        user_id: "user123".into(),
        email: "a@b.es".into(),
        token,
        psk: "cafe".repeat(16),
    };
    let result = pb.ensure_fresh(&session, 300).await.unwrap();
    assert!(result.is_none(), "fresh token must not trigger a refresh");
}

#[tokio::test]
async fn change_password_patches_user_record() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/collections/users/records/user123"))
        .and(header("authorization", "tok-1"))
        .and(body_json_string(
            r#"{"oldPassword":"old-pw","password":"new-pw","passwordConfirm":"new-pw"}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":"user123"})))
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let session = marea_auth::AuthSession {
        user_id: "user123".into(),
        email: "a@b.es".into(),
        token: "tok-1".into(),
        psk: String::new(),
    };
    pb.change_password(&session, "old-pw", "new-pw")
        .await
        .unwrap();
}

#[tokio::test]
async fn collection_crud_round_trip() {
    #[derive(serde::Serialize)]
    struct NewGroup<'a> {
        name: &'a str,
    }
    #[derive(serde::Deserialize)]
    struct Group {
        id: String,
        name: String,
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/collections/group/records"))
        .and(header("authorization", "tok-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":"g1","name":"Casa"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/collections/group/records"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page": 1, "totalItems": 1, "items": [{"id":"g1","name":"Casa"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/collections/group/records/g1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let pb = PocketBase::with_url(server.uri(), DOMAIN.to_vec());
    let col = pb.collection("group", Some("tok-1".into()));
    let created: Group = col.create(&NewGroup { name: "Casa" }).await.unwrap();
    assert_eq!(created.id, "g1");
    let listed = col
        .list::<Group>(Some("name='Casa'"), None, Some(1), None)
        .await
        .unwrap();
    assert_eq!(listed.total_items, 1);
    assert_eq!(listed.items[0].name, "Casa");
    col.delete("g1").await.unwrap();
}
