//! Real generated SaaS router, password controller, users and
//! password_reset_tokens tables on SQLite: a password change keeps only the
//! asking browser signed in, and an emailed reset token works once, expires,
//! is replaced by a newer request and ends every session.
use super::*;
use rullst::mail::{MailTrap, MemoryDriver, Message};
use rullst::server::{Body, Request, StatusCode};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};
use tower::ServiceExt;

const CSRF: &str = "PasswordFlowsContractCsrfToken01";
const MEMBER: &str = "member@example.test";
const OLD_PASSWORD: &str = "old-contract-password-1";
const NEW_PASSWORD: &str = "new-contract-password-2";
const RESET_PASSWORD: &str = "reset-contract-password-3";
/// A distinct peer per request, so the per-client credential budget never
/// throttles the contract.
static PEER: AtomicU8 = AtomicU8::new(1);

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).map_or("", |value| value.to_str().unwrap())
    }

    /// The `rullst_session` value this response sets, if any.
    fn session(&self) -> Option<String> {
        self.headers.get_all("set-cookie").iter().find_map(|value| {
            let value = value.to_str().unwrap().strip_prefix("rullst_session=")?;
            Some(value.split(';').next().unwrap().to_owned())
        })
    }
}

type App = rullst::web::axum::Router;

async fn call(app: &App, method: &str, uri: &str, session: Option<&str>, form: &str, host: &str) -> Reply {
    let mut cookie = format!("rullst_csrf={CSRF}");
    if let Some(session) = session {
        cookie.push_str(&format!("; rullst_session={session}"));
    }
    let peer = std::net::SocketAddr::from(([127, 0, 0, PEER.fetch_add(1, Ordering::Relaxed)], 40_000));
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", host)
        .header("cookie", cookie)
        .header("x-csrf-token", CSRF)
        .header("content-type", "application/x-www-form-urlencoded")
        .extension(rullst::web::axum::extract::ConnectInfo(peer))
        .body(Body::from(form.to_owned()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = rullst::web::axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap();
    Reply { status, headers, body: String::from_utf8(body.to_vec()).unwrap() }
}

async fn get(app: &App, uri: &str, session: Option<&str>) -> Reply {
    call(app, "GET", uri, session, "", "localhost:3000").await
}

async fn post(app: &App, uri: &str, session: Option<&str>, form: &str) -> Reply {
    call(app, "POST", uri, session, form, "localhost:3000").await
}

async fn stored_epoch() -> i64 {
    User::find(1).await.unwrap().unwrap().session_epoch
}

async fn password_is(password: &str) -> bool {
    let hash = User::find(1).await.unwrap().unwrap().password_hash.unwrap();
    rullst::auth::verify_password(password, &hash)
}

async fn reset_rows() -> Vec<(String, i64, i64)> {
    rullst::db::sqlx::query_as("SELECT token_hash, expires_at, created_at FROM password_reset_tokens")
        .fetch_all(rullst::orm::Orm::pool().unwrap())
        .await
        .unwrap()
}

async fn execute(sql: &'static str) {
    rullst::db::sqlx::query(sql).execute(rullst::orm::Orm::pool().unwrap()).await.unwrap();
}

/// The mail sent to `address` once `count` messages arrived, waiting at most
/// ten seconds for the background delivery.
async fn mail_to(address: &str, count: usize) -> Vec<Message> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let sent: Vec<Message> =
            MailTrap::sent_messages().into_iter().filter(|message| message.to == address).collect();
        if sent.len() >= count || Instant::now() > deadline {
            return sent;
        }
        rullst::runtime::time::sleep(Duration::from_millis(20)).await;
    }
}

fn link_token(message: &Message, origin: &str) -> String {
    let text = message.body_text.as_deref().unwrap();
    let prefix = format!("{origin}/password/reset?token=");
    let start = text.find(&prefix).unwrap() + prefix.len();
    text[start..start + 64].to_owned()
}

/// The page without its per-request CSP nonce.
fn without_nonce(body: &str) -> String {
    body.split("nonce=\"")
        .enumerate()
        .map(|(index, part)| if index == 0 { part } else { part.split_once('"').unwrap().1 })
        .collect()
}

#[tokio::test]
async fn generated_password_flows_contract() {
    rullst::orm::Orm::init("sqlite::memory:").await.unwrap();
    for migration in crate::migrations::get_migrations() {
        migration.up().await.unwrap();
    }
    let old_hash = rullst::auth::hash_password(OLD_PASSWORD).unwrap();
    rullst::db::sqlx::query("INSERT INTO users (id,name,email,password_hash,created_at,updated_at) VALUES (?,?,?,?,?,?)")
        .bind(1).bind("Member").bind(MEMBER).bind(old_hash).bind("2026-10-15T00:00:00+00:00").bind("2026-10-15T00:00:00+00:00")
        .execute(rullst::orm::Orm::pool().unwrap()).await.unwrap();
    rullst::mail::Mail::set_driver(Box::new(MemoryDriver::new()));
    MailTrap::clear();
    let key = rullst::auth::get_app_key().unwrap();
    let issue = |epoch| rullst::auth::encrypt_session_with_epoch(1, epoch, rullst::auth::DEFAULT_SESSION_MAX_AGE, &key).unwrap();
    let (laptop, phone) = (issue(0), issue(0));
    let router = crate::router().unwrap();
    let app = router.as_axum();

    // Change password: signed-in only, behind CSRF.
    assert_eq!(get(app, "/password/change", None).await.status, StatusCode::SEE_OTHER);
    let form = get(app, "/password/change", Some(&laptop)).await;
    assert_eq!(form.status, StatusCode::OK);
    assert!(form.body.contains("action=\"/password/change\""));
    let change = |current: &str, new: &str, confirmation: &str| {
        format!("_token={CSRF}&current_password={current}&password={new}&password_confirmation={confirmation}")
    };
    let wrong = post(app, "/password/change", Some(&laptop), &change("not the password 0", NEW_PASSWORD, NEW_PASSWORD)).await;
    assert_eq!(wrong.status, StatusCode::OK);
    assert!(wrong.body.contains("The current password is incorrect"));
    let mismatch = post(app, "/password/change", Some(&laptop), &change(OLD_PASSWORD, NEW_PASSWORD, RESET_PASSWORD)).await;
    assert!(mismatch.body.contains("The new password and its confirmation do not match"));
    let short = post(app, "/password/change", Some(&laptop), &change(OLD_PASSWORD, "short", "short")).await;
    assert!(short.body.contains("at least 12 characters"));
    assert_eq!(stored_epoch().await, 0);
    assert!(password_is(OLD_PASSWORD).await);

    // A reset link requested before the change stops working after it.
    post(app, "/password/forgot", None, &format!("_token={CSRF}&email={MEMBER}")).await;
    let early = link_token(&mail_to(MEMBER, 1).await[0], "http://localhost:3000");
    assert_eq!(reset_rows().await.len(), 1);

    let changed = post(app, "/password/change", Some(&laptop), &change(OLD_PASSWORD, NEW_PASSWORD, NEW_PASSWORD)).await;
    assert_eq!(changed.status, StatusCode::SEE_OTHER);
    assert_eq!(changed.header("location"), "/password/change?changed=1");
    let current = changed.session().unwrap();
    assert_eq!(stored_epoch().await, 1);
    assert!(password_is(NEW_PASSWORD).await);
    assert!(!password_is(OLD_PASSWORD).await);
    // This browser stays signed in; every other copy is revoked.
    assert_eq!(get(app, "/dashboard", Some(&current)).await.status, StatusCode::OK);
    for revoked in [&laptop, &phone] {
        assert_eq!(get(app, "/dashboard", Some(revoked)).await.status, StatusCode::SEE_OTHER);
    }
    // A revoked cookie cannot change the password again.
    let stale = post(app, "/password/change", Some(&phone), &change(NEW_PASSWORD, RESET_PASSWORD, RESET_PASSWORD)).await;
    assert_eq!(stale.status, StatusCode::SEE_OTHER);
    assert_eq!(stored_epoch().await, 1);
    assert!(get(app, "/password/change?changed=1", Some(&current)).await.body.contains("Your password was changed"));
    assert!(reset_rows().await.is_empty());
    assert!(get(app, &format!("/password/reset?token={early}"), None).await.body.contains("invalid or has expired"));
    let notices = mail_to(MEMBER, 2).await;
    assert_eq!(notices.len(), 2);
    assert!(notices[1].subject.ends_with(": Your password was changed"));
    MailTrap::clear();

    // Forgot password: an unknown email gets the same answer, no sooner.
    let started = Instant::now();
    let unknown = post(app, "/password/forgot", None, &format!("_token={CSRF}&email=nobody%40example.test")).await;
    assert!(started.elapsed() >= Duration::from_millis(400));
    let known = post(app, "/password/forgot", None, &format!("_token={CSRF}&email={MEMBER}")).await;
    assert_eq!((unknown.status, known.status), (StatusCode::OK, StatusCode::OK));
    assert!(known.body.contains("If an account uses that email address, we sent it a link"));
    assert_eq!(without_nonce(&unknown.body), without_nonce(&known.body));
    let first_mail = mail_to(MEMBER, 1).await;
    assert_eq!(first_mail.len(), 1);
    assert!(mail_to("nobody@example.test", 0).await.is_empty());
    let first = link_token(&first_mail[0], "http://localhost:3000");
    // Only the digest is stored, valid for 30 minutes.
    let rows = reset_rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(Some(rows[0].0.clone()), rullst::auth::password_reset_token_digest(&first));
    assert_ne!(rows[0].0, first);
    assert_eq!(rows[0].1 - rows[0].2, 1_800);

    // The emailed link opens the form, kept out of caches and Referer headers.
    let page = get(app, &format!("/password/reset?token={first}"), None).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains(&format!("name=\"token\" value=\"{first}\"")));
    assert_eq!(page.header("referrer-policy"), "no-referrer");
    assert_eq!(page.header("cache-control"), "no-store");

    // A tampered token is refused.
    let tampered = format!("{}{}", &first[..63], if first.ends_with('0') { '1' } else { '0' });
    let reset = |token: &str, password: &str, confirmation: &str| {
        format!("_token={CSRF}&token={token}&password={password}&password_confirmation={confirmation}")
    };
    assert!(get(app, &format!("/password/reset?token={tampered}"), None).await.body.contains("invalid or has expired"));
    let refused = post(app, "/password/reset", None, &reset(&tampered, RESET_PASSWORD, RESET_PASSWORD)).await;
    assert!(refused.body.contains("invalid or has expired"));
    assert_eq!(stored_epoch().await, 1);

    // Within the cooldown a repeated request keeps the live token and sends nothing.
    post(app, "/password/forgot", None, &format!("_token={CSRF}&email={MEMBER}")).await;
    assert_eq!(reset_rows().await[0].0, rullst::auth::password_reset_token_digest(&first).unwrap());

    // After it, a second request replaces the first token; its link uses the
    // configured origin, never the request's Host header.
    execute("UPDATE password_reset_tokens SET created_at = created_at - 61").await;
    // SAFETY: this contract runs alone in its process and no other thread reads the environment now.
    unsafe { std::env::set_var("RULLST_PUBLIC_ORIGIN", "https://accounts.example.test/") };
    let replaced = call(app, "POST", "/password/forgot", None, &format!("_token={CSRF}&email={MEMBER}"), "attacker.example.test").await;
    unsafe { std::env::remove_var("RULLST_PUBLIC_ORIGIN") };
    assert_eq!(replaced.status, StatusCode::OK);
    let second_mail = mail_to(MEMBER, 2).await;
    assert_eq!(second_mail.len(), 2);
    assert!(!second_mail[1].body_text.as_deref().unwrap().contains("attacker.example.test"));
    let second = link_token(&second_mail[1], "https://accounts.example.test");
    assert_ne!(second, first);
    let stale = post(app, "/password/reset", None, &reset(&first, RESET_PASSWORD, RESET_PASSWORD)).await;
    assert!(stale.body.contains("invalid or has expired"));

    // The registration policy and the confirmation apply; the token survives them.
    let weak = post(app, "/password/reset", None, &reset(&second, "short", "short")).await;
    assert!(weak.body.contains("at least 12 characters"));
    let mismatch = post(app, "/password/reset", None, &reset(&second, RESET_PASSWORD, NEW_PASSWORD)).await;
    assert!(mismatch.body.contains("do not match"));
    assert_eq!(reset_rows().await.len(), 1);

    // The reset stores the password, ends every session and signs nobody in.
    let done = post(app, "/password/reset", None, &reset(&second, RESET_PASSWORD, RESET_PASSWORD)).await;
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    assert_eq!(done.header("location"), "/login?reset=done");
    assert_eq!(done.session(), None);
    assert_eq!(stored_epoch().await, 2);
    assert!(password_is(RESET_PASSWORD).await);
    assert!(reset_rows().await.is_empty());
    assert_eq!(get(app, "/dashboard", Some(&current)).await.status, StatusCode::SEE_OTHER);
    assert!(get(app, "/login?reset=done", None).await.body.contains("Your password was reset"));
    assert!(mail_to(MEMBER, 3).await[2].subject.ends_with(": Your password was changed"));

    // The token works once.
    let reused = post(app, "/password/reset", None, &reset(&second, NEW_PASSWORD, NEW_PASSWORD)).await;
    assert!(reused.body.contains("invalid or has expired"));
    assert_eq!(stored_epoch().await, 2);
    assert!(password_is(RESET_PASSWORD).await);

    // An expired token is refused.
    post(app, "/password/forgot", None, &format!("_token={CSRF}&email={MEMBER}")).await;
    let expiring = link_token(&mail_to(MEMBER, 4).await[3], "http://localhost:3000");
    execute("UPDATE password_reset_tokens SET expires_at = created_at").await;
    assert!(get(app, &format!("/password/reset?token={expiring}"), None).await.body.contains("invalid or has expired"));
    let expired = post(app, "/password/reset", None, &reset(&expiring, NEW_PASSWORD, NEW_PASSWORD)).await;
    assert!(expired.body.contains("invalid or has expired"));
    assert_eq!(stored_epoch().await, 2);
    assert!(password_is(RESET_PASSWORD).await);

    // The new password signs in.
    let login = post(app, "/login", None, &format!("_token={CSRF}&email={MEMBER}&password={RESET_PASSWORD}")).await;
    assert_eq!(login.status, StatusCode::SEE_OTHER);
    assert_eq!(login.header("location"), "/dashboard");
}
