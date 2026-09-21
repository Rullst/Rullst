//! Run without credentials: cargo run -p rullst-media --example private_course_video --all-features
//! This exercises the real SQLite/service lifecycle with an explicitly offline adapter.
use rullst_media::{bunny::*, sqlite::*, *};
use std::sync::atomic::{AtomicBool, Ordering};

struct CourseAccess {
    scope: Scope,
    enrolled: AtomicBool,
}
impl Authorization for CourseAccess {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, MediaError> {
        // Replace this example policy with authoritative session/tenant/role and
        // enrollment reads. Never trust a role or enrollment sent by the browser.
        let allowed = scope == &self.scope
            && (actor.as_str() == "instructor"
                || (action == Action::Play
                    && actor.as_str() == "learner"
                    && self.enrolled.load(Ordering::SeqCst)));
        if !allowed {
            return Err(MediaError::Denied);
        }
        Permission::until(
            SystemClock
                .now()?
                .checked_add(300)
                .ok_or(MediaError::Clock)?,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let provider = BunnyStream::new(BunnyConfig::new(
        LibraryId::new(7)?,
        Reference::new("offline-example")?,
        "example.b-cdn.net",
        BunnyCredentials::new("mock_api", "mock_webhook", "mock_embed", "mock_cdn")?,
        // Explicit test configuration only; this does not verify a Bunny account.
        PrivateDelivery::configured(true, true, true)?,
    )?)?;
    let store = SqliteMedia::initialize(
        directory.path().join("media.sqlite"),
        StoreConfig::testing(provider.binding(), 100)?,
        SystemClock,
    )
    .await?;
    let media = MediaService::new(provider, store)?;
    let scope = Scope::new("school", "rust-course")?;
    let policy = CourseAccess {
        scope: scope.clone(),
        enrolled: AtomicBool::new(true),
    };
    let instructor = Reference::new("instructor")?;
    let learner = Reference::new("learner")?;
    // Persist this creation ID before retrying an ambiguous request.
    let id = Reference::new("lesson-one-v1")?;
    let created = media
        .create(
            &policy,
            &instructor,
            &scope,
            &id,
            Metadata::new("Ownership", "Read the accessible lesson transcript here.")?,
        )
        .await?;
    let video = created.video.as_ref().ok_or(MediaError::Protocol)?;
    let upload = media.upload(&policy, &instructor, &scope, &id, 120).await?;
    if upload.mode != ProviderMode::Offline {
        return Err(MediaError::Configuration.into());
    }
    // Offline grants are deliberately not usable in the browser. Only this
    // offline adapter allows explicit processing simulation; no upload occurs.
    media
        .provider()
        .simulate_processing(video, Processing::Ready)?;
    let current = media.get(&policy, &instructor, &scope, &id).await?;
    media
        .publish(&policy, &instructor, &scope, &id, current.revision)
        .await?;
    let playback = media
        .playback(&policy, &learner, &scope, &id, 60, PlaybackKind::Embed)
        .await?;
    if playback.mode != ProviderMode::Offline {
        return Err(MediaError::Configuration.into());
    }
    policy.enrolled.store(false, Ordering::SeqCst);
    if !matches!(
        media
            .playback(&policy, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await,
        Err(MediaError::Denied)
    ) {
        return Err(MediaError::Configuration.into());
    }
    let current = media.get(&policy, &instructor, &scope, &id).await?;
    let withdrawn = media
        .withdraw(&policy, &instructor, &scope, &id, current.revision)
        .await?;
    let deleted = media
        .delete(&policy, &instructor, &scope, &id, withdrawn.revision)
        .await?;
    if deleted.lifecycle != Lifecycle::Deleted {
        return Err(MediaError::Protocol.into());
    }
    media.close().await;
    println!(
        "Offline lifecycle passed: creation, upload grant, readiness, explicit publication, entitlement withdrawal and deletion. No provider was contacted."
    );
    Ok(())
}
