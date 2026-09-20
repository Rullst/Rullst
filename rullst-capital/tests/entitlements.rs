use rullst_capital::{BillingSubject, entitlements::*};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn scope(tenant: &str, owner: &str) -> EntitlementScope {
    EntitlementScope::new(tenant, BillingSubject::try_new("user", owner).unwrap()).unwrap()
}
fn policy(mode: EntitlementMode) -> EntitlementPolicy {
    EntitlementPolicy::new("reports.billing", ["price_pro"], mode, 30).unwrap()
}
fn snapshot(scope: EntitlementScope, mode: EntitlementMode) -> EntitlementSnapshot {
    EntitlementSnapshot::from_reconciled(
        scope,
        "price_pro",
        EntitlementStatus::Active,
        mode,
        100,
        200,
    )
    .unwrap()
}
struct Clock(Mutex<std::collections::VecDeque<Result<i64, EntitlementError>>>);
impl Clock {
    fn new(before: i64, after: i64) -> Self {
        Self(Mutex::new([Ok(before), Ok(after)].into()))
    }
}
impl EntitlementClock for Clock {
    fn unix_seconds(&self) -> Result<i64, EntitlementError> {
        self.0.lock().unwrap().pop_front().unwrap()
    }
}
struct Store {
    state: Mutex<Result<Option<EntitlementSnapshot>, EntitlementError>>,
    reads: AtomicUsize,
}
impl Store {
    fn new(value: Option<EntitlementSnapshot>) -> Self {
        Self {
            state: Mutex::new(Ok(value)),
            reads: AtomicUsize::new(0),
        }
    }
}
impl EntitlementStore for Store {
    async fn current(
        &self,
        _: &EntitlementScope,
    ) -> Result<Option<EntitlementSnapshot>, EntitlementError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.state.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn reads_current_state_each_time_and_revocation_is_effective() {
    let scope = scope("school", "owner");
    let store = Store::new(Some(snapshot(scope.clone(), EntitlementMode::Live)));
    let gate = EntitlementGate::new(policy(EntitlementMode::Live));
    assert_eq!(gate.feature(), "reports.billing");
    gate.authorize_with_clock(&scope, &store, &Clock::new(100, 129))
        .await
        .unwrap();
    *store.state.lock().unwrap() = Ok(Some(
        EntitlementSnapshot::from_reconciled(
            scope.clone(),
            "price_pro",
            EntitlementStatus::Revoked,
            EntitlementMode::Live,
            100,
            200,
        )
        .unwrap(),
    ));
    assert_eq!(
        gate.authorize_with_clock(&scope, &store, &Clock::new(100, 101))
            .await,
        Err(EntitlementError::Denied)
    );
    *store.state.lock().unwrap() = Ok(None);
    assert_eq!(
        gate.authorize_with_clock(&scope, &store, &Clock::new(100, 101))
            .await,
        Err(EntitlementError::Denied)
    );
    *store.state.lock().unwrap() = Err(EntitlementError::Unavailable);
    assert_eq!(
        gate.authorize_with_clock(&scope, &store, &Clock::new(100, 101))
            .await,
        Err(EntitlementError::Unavailable)
    );
    assert_eq!(store.reads.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn tenant_subject_kind_and_plan_must_match() {
    let requested = scope("tenant_a", "owner_a");
    let gate = EntitlementGate::new(policy(EntitlementMode::Live));
    for bound in [
        scope("tenant_b", "owner_a"),
        scope("tenant_a", "owner_b"),
        EntitlementScope::new(
            "tenant_a",
            BillingSubject::try_new("workspace", "owner_a").unwrap(),
        )
        .unwrap(),
    ] {
        let store = Store::new(Some(snapshot(bound, EntitlementMode::Live)));
        assert_eq!(
            gate.authorize_with_clock(&requested, &store, &Clock::new(100, 100))
                .await,
            Err(EntitlementError::Denied)
        );
    }
    for plan in ["price_basic", "PRICE_PRO", "price_pro.extra"] {
        let state = EntitlementSnapshot::from_reconciled(
            requested.clone(),
            plan,
            EntitlementStatus::Active,
            EntitlementMode::Live,
            100,
            200,
        )
        .unwrap();
        assert_eq!(
            gate.authorize_with_clock(&requested, &Store::new(Some(state)), &Clock::new(100, 100))
                .await,
            Err(EntitlementError::Denied)
        );
    }
}

#[tokio::test]
async fn only_active_status_and_exact_mode_authorize() {
    let requested = scope("tenant", "owner");
    for mode in [EntitlementMode::Live, EntitlementMode::Sandbox] {
        let gate = EntitlementGate::new(policy(mode));
        for source in [
            EntitlementMode::Live,
            EntitlementMode::Sandbox,
            EntitlementMode::Mock,
        ] {
            for status in [
                EntitlementStatus::Active,
                EntitlementStatus::Trial,
                EntitlementStatus::PastDue,
                EntitlementStatus::Revoked,
                EntitlementStatus::Other,
            ] {
                let state = EntitlementSnapshot::from_reconciled(
                    requested.clone(),
                    "price_pro",
                    status,
                    source,
                    100,
                    200,
                )
                .unwrap();
                let actual = gate
                    .authorize_with_clock(
                        &requested,
                        &Store::new(Some(state)),
                        &Clock::new(100, 100),
                    )
                    .await;
                let expected = if source == mode && status == EntitlementStatus::Active {
                    Ok(())
                } else {
                    Err(EntitlementError::Denied)
                };
                assert_eq!(actual, expected, "{mode:?}/{source:?}/{status:?}");
            }
        }
    }
}

#[tokio::test]
async fn deadline_and_observation_age_are_checked_after_slow_reads() {
    let scope = scope("tenant", "owner");
    let gate = EntitlementGate::new(policy(EntitlementMode::Live));
    for (before, after, observed, ends, expected) in [
        (100, 100, 100, 101, Ok(())),
        (100, 101, 100, 101, Err(EntitlementError::Denied)),
        (100, 130, 100, 200, Err(EntitlementError::Denied)),
        (100, 100, 101, 200, Err(EntitlementError::Denied)),
        (100, 100, 100, 99, Err(EntitlementError::Denied)),
        (100, 99, 98, 200, Err(EntitlementError::Clock)),
        (-1, 100, 100, 200, Err(EntitlementError::Clock)),
        (
            i64::MAX - 1,
            i64::MAX,
            0,
            i64::MAX,
            Err(EntitlementError::Denied),
        ),
    ] {
        let state = EntitlementSnapshot::from_reconciled(
            scope.clone(),
            "price_pro",
            EntitlementStatus::Active,
            EntitlementMode::Live,
            observed,
            ends,
        )
        .unwrap();
        assert_eq!(
            gate.authorize_with_clock(&scope, &Store::new(Some(state)), &Clock::new(before, after))
                .await,
            expected
        );
    }
    for readings in [
        [Err(EntitlementError::Clock), Ok(100)],
        [Ok(100), Err(EntitlementError::Clock)],
    ] {
        let clock = Clock(Mutex::new(readings.into()));
        assert_eq!(
            gate.authorize_with_clock(
                &scope,
                &Store::new(Some(snapshot(scope.clone(), EntitlementMode::Live))),
                &clock
            )
            .await,
            Err(EntitlementError::Clock)
        );
    }
}

#[test]
fn configuration_bounds_are_validated_and_private_identity_is_redacted() {
    for tenant in ["", "../tenant", "a:b", "tenant space", "tenant\n"] {
        assert!(
            EntitlementScope::new(
                tenant,
                BillingSubject::try_new("user", "secret_owner").unwrap()
            )
            .is_err()
        );
    }
    for feature in ["", " ", "feature/../x"] {
        assert!(EntitlementPolicy::new(feature, ["price_pro"], EntitlementMode::Live, 30).is_err());
    }
    for plans in [
        vec![],
        vec!["price_pro", "price_pro"],
        vec![""],
        vec!["price/../x"],
    ] {
        assert!(
            EntitlementPolicy::new("reports.billing", plans, EntitlementMode::Live, 30).is_err()
        );
    }
    assert!(
        EntitlementPolicy::new(
            "reports.billing",
            (0..65).map(|n| format!("price_{n}")),
            EntitlementMode::Live,
            30
        )
        .is_err()
    );
    assert!(
        EntitlementPolicy::new(
            "reports.billing",
            (0..64).map(|n| format!("price_{n}")),
            EntitlementMode::Live,
            300
        )
        .is_ok()
    );
    assert!(
        EntitlementPolicy::new("reports.billing", ["price_pro"], EntitlementMode::Mock, 30)
            .is_err()
    );
    for age in [0, 301, u16::MAX] {
        assert!(
            EntitlementPolicy::new("reports.billing", ["price_pro"], EntitlementMode::Live, age)
                .is_err()
        );
    }
    for (at, until) in [(-1, 200), (100, 0)] {
        assert!(
            EntitlementSnapshot::from_reconciled(
                scope("secret_tenant", "secret_owner"),
                "price_secret",
                EntitlementStatus::Active,
                EntitlementMode::Live,
                at,
                until
            )
            .is_err()
        );
    }
    let scope = scope("secret_tenant", "secret_owner");
    let state = snapshot(scope.clone(), EntitlementMode::Live);
    for text in [
        format!("{scope:?}"),
        format!("{state:?}"),
        format!("{:?}", policy(EntitlementMode::Live)),
    ] {
        assert!(!text.contains("secret_"));
        assert!(!text.contains("price_pro"));
    }
    assert_eq!(scope.tenant(), "secret_tenant");
    assert_eq!(scope.subject().id(), "secret_owner");
}

#[tokio::test]
async fn normal_gate_uses_system_time_and_rejects_expired_state() {
    assert!(SystemEntitlementClock.unix_seconds().unwrap() > 0);
    let scope = scope("tenant", "owner");
    assert_eq!(
        EntitlementGate::new(policy(EntitlementMode::Live))
            .authorize(
                &scope,
                &Store::new(Some(snapshot(scope.clone(), EntitlementMode::Live)))
            )
            .await,
        Err(EntitlementError::Denied)
    );
}
