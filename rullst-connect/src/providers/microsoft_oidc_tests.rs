//! Signed, network-free tests of the Microsoft OpenID Connect provider.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::client::{HttpRequest, HttpResponse};
use serde_json::json;
use std::sync::Mutex;

// Public, test-only fixture shared with the other ID-token tests.
const RSA_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----\n\
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDUJ5CJ1uq6Fa6u\n\
D2dVnj5f4QYm6xN5oiFxJeISHSMIfL8cQQtOwPpW73VomiiuUw0f7Qa6ermrLe+z\n\
oHQLv003Sx4PUL68wxObVFXQPFInUwbLHbqDNzZt7idEtrK3n0Bytjol7n/g3CRf\n\
0icl0kYcO1YbswfJvmJzjOKXVesK9s5CgugaL6dR7TIdJ8gboICYAf5k7T9pPAMV\n\
ELz5yEM2SajzQhiU1K9Y0bH0wzBC1aRpBVLU5z9iG7AO4EJ5CGyi5IQcJciYgp1H\n\
GCFWKGY2NOQ2besmWUFOPHJe+EZEixq87cXod8Llx5nZtqYoPlomgYciHEj9jpFF\n\
nINRvvK5AgMBAAECggEAa4AAxVeZaOFDuf8kJUYh5QNo0p+bJq74sxS3EOaisdJE\n\
JFTxHd66+wIrQ+2ZX3vF0r+QAT3ehtan9yT+qFUvEy2E9c28WHmgnbyGHxXxqutv\n\
LczKjWKUue9LBo5s0I5pYbbkkAPh2Fa0N7mNDKUX0YZfg3mcIKXPzS0+Q+DNUKgC\n\
d38wKKLBKlqLbsPU3KVkL2hoTnZTp85NoXxbnMCrtrodjDrEjdcgvVeWeTVFqO/q\n\
haA11bPIKxu5Gll9zCy9VkdRdHjNkEI9Qcld6Lw2MTU/bDwvuQGsr6IxAz2or+Ib\n\
tzEJJDNXir3DP2iCdyUaBabo5cnYy6AAQ6gxuFYEAQKBgQDsjzVHA9Bbrhw2XGZ/\n\
2sZIV4GxKuGuNWWm/CMAh7u34JnO2xKj5kSAi3rEkFAiWEclFZlxVlnYNx8dYdh4\n\
aM7BMw0cSrzKSc7Xa2eAjQ0K2p+vx+u2QhJy/qVRESG0egmcsGWAwrdBPGpqKHYH\n\
wAki3dqzMlRgZMhCgtBpxISQ8QKBgQDlluvhyFW48d1F7yNQBTLGLwM7uHurnMms\n\
09c/jSbQGn+nOlli+jTLyMX4DjQIz6+2O5hR0KALWNcq1dZTy3EKFHgvqtkJF4Hm\n\
RDwCRcqDZxbFx+ewIj79ycVhB+mYiD+pJcf5f1Ogc1w3DZuhzTrRsGBo5lLtjM3d\n\
AoAzskt+SQKBgC+szPf69MsFU/pAtQefd8asnB6wnbsWV95HgmZg9JwiT904mZEe\n\
nz+o3J0w2HWThQMcT0hgNss0kLjDN3VM6h5Vw5aoGVRLe7w+kSV/R9mgJf6vM/oP\n\
Zth2Kask4L4Wukkx48MHexdSrb+nV+JH+Y9lVuY2hnrG1PVSl945FN6BAoGBAK98\n\
bS31/7fOfzBOOjKW1plvI8yJFVY2EFzeyz8TN+CG8J201s/1qVc+TjttN86oWIk1\n\
Ahc/HKWvsT9XlWwVK4Dl5nug3iW55xtHeorOJ53KtThVtT0G4BkCGbEx6BYjxm0W\n\
qMSG0zfoFUsrRpMlGFlgtEBaFHboUg4lNDLPjC6pAoGAFpXBekgzkUMfTpDQGJNX\n\
8aFTRTMSQKuPPq3B4UeJI+tVlSUdHCXIa7oGBTLYIWMCgWjpbDaU/Fkczljlk6d2\n\
TxJdI1XtSKNCfaAPkAND44lK1zdpnnImQQA8/r7ohOSUTfMT98q2atLHRbmldnwg\n\
z1F4IZ42Gry2+4guKvvM+O8=\n\
-----END PRIVATE KEY-----";
const RSA_N: &str = "1CeQidbquhWurg9nVZ4-X-EGJusTeaIhcSXiEh0jCHy_HEELTsD6Vu91aJoorlMNH-0Gunq5qy3vs6B0C79NN0seD1C-vMMTm1RV0DxSJ1MGyx26gzc2be4nRLayt59AcrY6Je5_4NwkX9InJdJGHDtWG7MHyb5ic4zil1XrCvbOQoLoGi-nUe0yHSfIG6CAmAH-ZO0_aTwDFRC8-chDNkmo80IYlNSvWNGx9MMwQtWkaQVS1Oc_YhuwDuBCeQhsouSEHCXImIKdRxghVihmNjTkNm3rJllBTjxyXvhGRIsavO3F6HfC5ceZ2bamKD5aJoGHIhxI_Y6RRZyDUb7yuQ";
const TENANT: &str = "72f988bf-86f1-41af-91ab-2d7cd011db47";
const OTHER_TENANT: &str = "11111111-2222-3333-4444-555555555555";
const OID: &str = "00000000-0000-0000-66f3-3332eca7ea81";
const NONCE: &str = "nonce-of-this-attempt";

/// Answers the token and key requests; records every URL and token form.
struct Entra {
    token: Value,
    urls: Mutex<Vec<String>>,
    forms: Mutex<Vec<String>>,
}

#[async_trait]
impl HttpClient for Entra {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
        self.urls.lock().unwrap().push(request.url.clone());
        let body = if request.url.ends_with("/discovery/v2.0/keys") {
            json!({"keys": [{"kid": "ms-key", "kty": "RSA", "alg": "RS256", "use": "sig", "n": RSA_N, "e": "AQAB"}]})
        } else {
            self.forms
                .lock()
                .unwrap()
                .push(request.form.unwrap_or_default());
            self.token.clone()
        };
        Ok(HttpResponse { status: 200, body })
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn claims(tid: &str) -> Value {
    json!({
        "iss": format!("https://login.microsoftonline.com/{tid}/v2.0"),
        "aud": "client-id",
        "sub": "pairwise-subject",
        "tid": tid,
        "oid": OID,
        "iat": now(),
        "exp": now() + 3600,
        "nonce": NONCE,
        "name": "Megan Bowen",
        "email": "megan@contoso.example",
    })
}

fn signed(claims: &Value, algorithm: jsonwebtoken::Algorithm, kid: Option<&str>) -> String {
    let mut header = jsonwebtoken::Header::new(algorithm);
    header.kid = kid.map(str::to_owned);
    let key = match algorithm {
        jsonwebtoken::Algorithm::HS256 => jsonwebtoken::EncodingKey::from_secret(b"shared"),
        _ => jsonwebtoken::EncodingKey::from_rsa_pem(RSA_KEY).unwrap(),
    };
    jsonwebtoken::encode(&header, claims, &key).unwrap()
}

fn token_response(id_token: Option<String>) -> Value {
    let mut token = json!({
        "access_token": "graph-access-token",
        "refresh_token": "refresh-token",
        "expires_in": 3599,
    });
    if let Some(id_token) = id_token {
        token["id_token"] = json!(id_token);
    }
    token
}

fn provider(tenant: MicrosoftTenant, token: Value) -> (MicrosoftOidcProvider, Arc<Entra>) {
    let entra = Arc::new(Entra {
        token,
        urls: Mutex::new(Vec::new()),
        forms: Mutex::new(Vec::new()),
    });
    let provider = MicrosoftOidcProvider::try_new(
        tenant,
        "client-id",
        SecretString::from("client-secret".to_owned()),
        "https://app.example/auth/microsoft/callback",
    )
    .unwrap()
    .with_http_client(entra.clone());
    (provider, entra)
}

async fn sign_in(tenant: MicrosoftTenant, claims: &Value) -> Result<ConnectUser, ConnectError> {
    let token = token_response(Some(signed(
        claims,
        jsonwebtoken::Algorithm::RS256,
        Some("ms-key"),
    )));
    let (provider, _) = provider(tenant, token);
    provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            code_verifier: Some("verifier"),
            expected_nonce: Some(NONCE),
        })
        .await
}

fn tenant(id: &str) -> MicrosoftTenant {
    MicrosoftTenant::Tenant(id.to_owned())
}

#[test]
fn tenants_parse_from_configuration() {
    assert_eq!(
        MicrosoftTenant::parse("common").unwrap(),
        MicrosoftTenant::Common
    );
    assert_eq!(
        MicrosoftTenant::parse(" Organizations ").unwrap(),
        MicrosoftTenant::Organizations
    );
    assert_eq!(
        MicrosoftTenant::parse("CONSUMERS").unwrap(),
        MicrosoftTenant::Consumers
    );
    assert_eq!(
        MicrosoftTenant::parse("72F988BF-86F1-41AF-91AB-2D7CD011DB47").unwrap(),
        tenant(TENANT)
    );
    for refused in [
        "",
        "contoso.onmicrosoft.com",
        "72f988bf-86f1-41af-91ab-2d7cd011db4",
        "72f988bf-86f1-41af-91ab-2d7cd011db477",
        "72f988bf86f1-41af-91ab-2d7cd011db47a",
        "72f988bf-86f1-41af-91ab2d7cd011db47",
        "72f988bf-86f1-41af-91ab-2d7cd011db4g",
        "72f988bf-86f1-41af-91ab-2d7cd011db47-0",
    ] {
        assert!(
            matches!(
                MicrosoftTenant::parse(refused),
                Err(ConnectError::InvalidConfiguration {
                    field: "tenant",
                    ..
                })
            ),
            "{refused}"
        );
    }
    assert_eq!(MicrosoftTenant::Common.as_str(), "common");
    assert_eq!(MicrosoftTenant::Organizations.as_str(), "organizations");
    assert_eq!(MicrosoftTenant::Consumers.as_str(), "consumers");
    assert_eq!(tenant(TENANT).as_str(), TENANT);
}

#[test]
fn each_tenant_admits_its_accounts() {
    let consumer = MICROSOFT_CONSUMER_TENANT_ID;
    assert_eq!(consumer, "9188040d-6c67-4c5b-b112-36a304b66dad");
    assert!(MicrosoftTenant::Common.admits(TENANT) && MicrosoftTenant::Common.admits(consumer));
    assert!(MicrosoftTenant::Organizations.admits(TENANT));
    assert!(!MicrosoftTenant::Organizations.admits(consumer));
    assert!(MicrosoftTenant::Consumers.admits(consumer));
    assert!(!MicrosoftTenant::Consumers.admits(TENANT));
    assert!(tenant(TENANT).admits(TENANT));
    assert!(!tenant(TENANT).admits(OTHER_TENANT));
}

#[test]
fn the_authorization_request_names_the_tenant_and_openid_scopes() {
    let (provider, _) = provider(MicrosoftTenant::Organizations, json!({}));
    assert_eq!(provider.tenant(), &MicrosoftTenant::Organizations);
    assert_eq!(provider.credential_mode(), CredentialMode::Live);
    assert_eq!(
        provider.redirect_url(),
        "https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize?\
         client_id=client-id&redirect_uri=https%3A%2F%2Fapp.example%2Fauth%2Fmicrosoft%2Fcallback\
         &scope=openid+profile+email&response_type=code&response_mode=query"
    );
    assert_eq!(
        provider.token_url(),
        "https://login.microsoftonline.com/organizations/oauth2/v2.0/token"
    );
}

#[tokio::test]
async fn a_verified_id_token_signs_in_tid_and_oid() {
    let mut verified = claims(TENANT);
    verified["xms_edov"] = json!(true);
    let token = token_response(Some(signed(
        &verified,
        jsonwebtoken::Algorithm::RS256,
        Some("ms-key"),
    )));
    let (provider, entra) = provider(tenant(TENANT), token);
    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "the-code",
            code_verifier: Some("the-verifier"),
            expected_nonce: Some(NONCE),
        })
        .await
        .unwrap();
    assert_eq!(user.id, format!("{TENANT}:{OID}"));
    assert_eq!(user.name, "Megan Bowen");
    assert_eq!(user.email.as_deref(), Some("megan@contoso.example"));
    assert_eq!(user.email_verified, Some(true));
    assert_eq!(user.avatar_url, None);
    assert_eq!(user.raw_data["tid"], TENANT);
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&user.access_token),
        "graph-access-token"
    );
    assert_eq!(
        user.refresh_token
            .map(|token| secrecy::ExposeSecret::expose_secret(&token).to_owned()),
        Some("refresh-token".to_owned())
    );
    assert_eq!(user.expires_in, Some(3599));
    assert_eq!(
        *entra.urls.lock().unwrap(),
        [
            format!("https://login.microsoftonline.com/{TENANT}/oauth2/v2.0/token"),
            format!("https://login.microsoftonline.com/{TENANT}/discovery/v2.0/keys"),
        ]
    );
    let form = entra.forms.lock().unwrap()[0].clone();
    for pair in [
        "client_id=client-id",
        "client_secret=client-secret",
        "code=the-code",
        "grant_type=authorization_code",
        "code_verifier=the-verifier",
    ] {
        assert!(form.contains(pair), "{pair}");
    }
}

#[tokio::test]
async fn the_email_is_verified_only_with_xms_edov() {
    let unverified = sign_in(MicrosoftTenant::Common, &claims(TENANT))
        .await
        .unwrap();
    assert_eq!(unverified.email.as_deref(), Some("megan@contoso.example"));
    assert_eq!(unverified.email_verified, None);
    for value in [json!(false), json!("false"), json!(1)] {
        let mut claims = claims(TENANT);
        claims["xms_edov"] = value;
        let user = sign_in(MicrosoftTenant::Common, &claims).await.unwrap();
        assert_eq!(user.email_verified, None);
    }
    let mut claims = claims(TENANT);
    claims["xms_edov"] = json!("true");
    claims.as_object_mut().unwrap().remove("name");
    let user = sign_in(MicrosoftTenant::Common, &claims).await.unwrap();
    assert_eq!(user.email_verified, Some(true));
    assert_eq!(user.name, "");
}

#[tokio::test]
async fn personal_accounts_follow_the_tenant_choice() {
    let consumer = claims(MICROSOFT_CONSUMER_TENANT_ID);
    for admitted in [MicrosoftTenant::Common, MicrosoftTenant::Consumers] {
        let user = sign_in(admitted, &consumer).await.unwrap();
        assert_eq!(user.id, format!("{MICROSOFT_CONSUMER_TENANT_ID}:{OID}"));
    }
    for refused in [MicrosoftTenant::Organizations, tenant(TENANT)] {
        let error = sign_in(refused, &consumer).await.unwrap_err().to_string();
        assert!(error.contains("directory may not sign in here"), "{error}");
    }
    let error = sign_in(MicrosoftTenant::Consumers, &claims(TENANT))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("directory may not sign in here"), "{error}");
    let error = sign_in(tenant(TENANT), &claims(OTHER_TENANT))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("directory may not sign in here"), "{error}");
}

#[tokio::test]
async fn forged_or_incomplete_tokens_are_refused() {
    let refused = |claims: Value, expected: &'static str| async move {
        let error = sign_in(MicrosoftTenant::Common, &claims)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    };
    // Another directory's issuer with this directory's tid.
    let mut claims_value = claims(TENANT);
    claims_value["iss"] = json!(format!(
        "https://login.microsoftonline.com/{OTHER_TENANT}/v2.0"
    ));
    refused(claims_value, "issuer does not match").await;
    let mut claims_value = claims(TENANT);
    claims_value["iss"] = json!(format!("https://sts.windows.net/{TENANT}/"));
    refused(claims_value, "issuer does not match").await;
    let mut claims_value = claims(TENANT);
    claims_value.as_object_mut().unwrap().remove("tid");
    refused(claims_value, "no directory (tenant) ID").await;
    let mut claims_value = claims(TENANT);
    claims_value["tid"] = json!("contoso");
    claims_value["iss"] = json!("https://login.microsoftonline.com/contoso/v2.0");
    refused(claims_value, "no directory (tenant) ID").await;
    let mut claims_value = claims(TENANT);
    claims_value.as_object_mut().unwrap().remove("oid");
    refused(claims_value, "no object ID").await;
    let mut claims_value = claims(TENANT);
    claims_value["oid"] = json!("");
    refused(claims_value, "no object ID").await;
    let mut claims_value = claims(TENANT);
    claims_value["aud"] = json!("another-client");
    refused(claims_value, "ID token validation failed").await;
    let mut claims_value = claims(TENANT);
    claims_value["exp"] = json!(now() - 3600);
    refused(claims_value, "ID token validation failed").await;
    let mut claims_value = claims(TENANT);
    claims_value["nonce"] = json!("another-attempt");
    assert!(
        sign_in(MicrosoftTenant::Common, &claims_value)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn unsigned_keyless_or_missing_id_tokens_are_refused() {
    for (id_token, expected) in [
        (
            Some(signed(
                &claims(TENANT),
                jsonwebtoken::Algorithm::HS256,
                Some("ms-key"),
            )),
            "unsupported ID token algorithm",
        ),
        (
            Some(signed(
                &claims(TENANT),
                jsonwebtoken::Algorithm::RS256,
                None,
            )),
            "no key ID",
        ),
        (None, "no ID token"),
    ] {
        let (provider, _) = provider(MicrosoftTenant::Common, token_response(id_token));
        let error = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "code",
                expected_nonce: Some(NONCE),
                ..Default::default()
            })
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let (provider, _) = provider(MicrosoftTenant::Common, json!({}));
    assert!(provider.get_user_from_token("access-token").await.is_err());
}

#[tokio::test]
async fn mock_credentials_sign_in_offline_in_the_configured_directory() {
    for (tenant_choice, tid) in [
        (tenant(TENANT), TENANT),
        (MicrosoftTenant::Consumers, MICROSOFT_CONSUMER_TENANT_ID),
        (
            MicrosoftTenant::Organizations,
            "00000000-0000-0000-0000-00000000000a",
        ),
    ] {
        let provider = MicrosoftOidcProvider::try_new(
            tenant_choice,
            "mock_microsoft",
            SecretString::from("mock_microsoft".to_owned()),
            "http://localhost:3000/auth/microsoft/callback",
        )
        .unwrap();
        assert!(provider.credential_mode().is_mock());
        let original = provider.http_client.clone();
        let provider = provider.with_http_client(Arc::new(Entra {
            token: json!({}),
            urls: Mutex::new(Vec::new()),
            forms: Mutex::new(Vec::new()),
        }));
        assert!(Arc::ptr_eq(&provider.http_client, &original));
        assert_eq!(
            provider.redirect_url(),
            "http://localhost:3000/auth/microsoft/callback?code=mock_authorization_code"
        );
        let user = provider
            .get_user(crate::provider::ExchangeParams {
                auth_code: "mock_authorization_code",
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            user.id,
            format!("{tid}:00000000-0000-0000-0000-000000000001")
        );
        assert_eq!(user.email.as_deref(), Some("mock@example.invalid"));
        assert_eq!(user.email_verified, Some(true));
        assert_eq!(user.name, "Rullst Mock User");
    }
    assert!(
        MicrosoftOidcProvider::try_new(
            MicrosoftTenant::Common,
            "client-id",
            SecretString::from("secret".to_owned()),
            "http://example.com/callback",
        )
        .is_err()
    );
}
