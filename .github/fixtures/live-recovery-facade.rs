use rullst::live::recovery::{
    LIVE_RECOVERY_MODULE, LiveCommand, LiveRecovery, LiveRecoveryConfig,
    LiveRecoveryError, LiveResult, LiveScope, LiveSnapshot, RecoverableLiveView,
};
use rullst::security::TenantMembership;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

// This consumer proves the public packaged boundary without application secrets
// or a database dependency. The full protocol/Chromium suites own transport and
// durable domain acceptance; this read-only view deliberately denies mutations.
struct View(Arc<AtomicBool>);
impl RecoverableLiveView for View {
    async fn authorize(&self, scope: &LiveScope) -> LiveResult<()> {
        if self.0.load(Ordering::Acquire) && scope.tenant() == "academy-a"
            && scope.account() == "learner-42" && scope.component() == "lesson/7" {
            Ok(())
        } else { Err(LiveRecoveryError::Unauthorized) }
    }
    async fn snapshot(&self, scope: &LiveScope) -> LiveResult<LiveSnapshot> {
        self.authorize(scope).await?;
        LiveSnapshot::try_new(42, "<p>Reviewed lesson</p>")
    }
    async fn apply(&self, scope: &LiveScope, _: &LiveCommand) -> LiveResult<LiveSnapshot> {
        self.authorize(scope).await?;
        Err(LiveRecoveryError::Unauthorized)
    }
}

#[test]
fn packaged_facade_exposes_recoverable_view_and_embedded_browser_module() {
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all().build().unwrap();
    runtime.block_on(async {
        let tenant = TenantMembership::try_new(["academy-a"]).unwrap().select("academy-a").unwrap();
        let scope = LiveScope::try_new(&tenant, "learner-42", "lesson/7").unwrap();
        let live = LiveRecovery::new(LiveRecoveryConfig::try_new("https://academy.example").unwrap());
        let _shared = live.clone();
        let active = Arc::new(AtomicBool::new(true));
        let view = View(active.clone());
        let state = view.snapshot(&scope).await.unwrap();
        assert_eq!(state.revision(), 42);
        assert_eq!(state.html(), "<p>Reviewed lesson</p>");
        let foreign = LiveScope::try_new(&tenant, "learner-99", "lesson/7").unwrap();
        assert!(matches!(view.snapshot(&foreign).await, Err(LiveRecoveryError::Unauthorized)));
        active.store(false, Ordering::Release);
        assert!(matches!(view.snapshot(&scope).await, Err(LiveRecoveryError::Unauthorized)));
        assert!(LIVE_RECOVERY_MODULE.contains("export function connectLive("));
        assert!(LIVE_RECOVERY_MODULE.contains("rullst.live.v1"));
    });
}
