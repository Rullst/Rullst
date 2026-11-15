use super::*;

async fn body(provider: &'static str, method: &'static str, url: &str) -> Value {
    let response = OfflineHttpClient::new(provider)
        .execute(HttpRequest {
            method: method.into(),
            url: url.to_string(),
            headers: reqwest::header::HeaderMap::new(),
            form: None,
            json: None,
            basic_auth: None,
            bearer_auth: None,
        })
        .await
        .expect("offline response");
    assert_eq!(response.status, 200);
    response.body
}

#[tokio::test]
async fn token_responses_answer_only_token_posts() {
    let token = OfflineHttpClient::token_response();
    assert_eq!(
        body("oidc", "POST", "https://idp.example/oauth/token").await,
        token
    );
    assert_eq!(
        body(
            "github",
            "POST",
            "https://github.example/login/oauth/access_token"
        )
        .await,
        token
    );
    // A token-like path read with GET and a POST elsewhere get the profile.
    let profile = OfflineHttpClient::new("oidc").user_response();
    assert_eq!(
        body("oidc", "GET", "https://idp.example/tokeninfo").await,
        profile
    );
    assert_eq!(
        body("oidc", "POST", "https://idp.example/userinfo").await,
        profile
    );
}

#[tokio::test]
async fn key_sets_are_served_for_jwks_certs_and_keys_paths() {
    let empty = json!({ "keys": [] });
    for url in [
        "https://idp.example/.well-known/jwks.json",
        "https://www.googleapis.example/oauth2/v3/certs",
        "https://login.example/common/discovery/keys",
    ] {
        assert_eq!(body("oidc", "GET", url).await, empty, "{url}");
    }
}

#[tokio::test]
async fn x_profiles_have_the_data_envelope() {
    for provider in ["x", "XProvider"] {
        let profile = body(provider, "GET", "https://api.x.example/2/users/me").await;
        assert_eq!(profile["data"]["id"], "mock-user", "{provider}");
    }
    let other = body("oidc", "GET", "https://idp.example/userinfo").await;
    assert_eq!(other["sub"], "mock-user");
    assert!(other.get("data").is_none());
}
