//! Minimal **non-executing** "bring your own runner" controller for Rullst Labs.
//!
//! Nothing here compiles, interprets or spawns learner code. It demonstrates
//! the controller side of the contract documented in
//! `docs/src/labs-runner-contract.md`: startup reconciliation, `claim_next`,
//! lease monitoring, a signed `ExecutionReceipt` sent to `complete`, and the
//! `abandon_attempt`/`reconcile_cleanup` path. The two `PLUG-IN POINT`
//! functions mark where an application-owned, separately deployed and isolated
//! runner would do real work.
//!
//! The application half (store creation, exercise registration and a learner
//! submission) is folded in so the example runs on its own in a temporary
//! directory. In a deployment those calls live in the web application, while
//! the controller is a separate process that opens the same shared-local store
//! and alone holds the receipt signing seed.
use rullst_labs::{
    sqlite::{CleanupJob, ContentKey, JobView, LeaseStatus, LeasedJob, SqliteLabs, StoreConfig},
    *,
};
use std::{path::PathBuf, time::Duration};
use zeroize::Zeroizing;

/// What the isolated runner hands back to the trusted controller.
struct IsolatedAttempt {
    output: WorkerOutput,
    /// Digest of the runner's actual per-job isolation/resource observations.
    observations: ContentHash,
}

/// PLUG-IN POINT: execute one attempt in YOUR isolated runner.
///
/// A real runner, in a separate process or host owned by the operator:
/// 1. launches a fresh worker under its own isolation boundary (no network, no
///    job database, no content key, no signing seed, no grader, no control
///    socket, enforced CPU/memory/process/disk/output limits) and verifies
///    those restrictions before releasing any source;
/// 2. writes the bounded `WorkerInput` JSON (source, case inputs, limits and
///    the opaque binding; never expected answers) to the worker;
/// 3. reads at most 20_480 reply bytes and decodes them with
///    `WorkerOutput::from_bytes`, which validates the outcome shape;
/// 4. tears the whole worker group and workspace down before returning.
///
/// This placeholder does none of that. It never reads `input.source()` and
/// answers with the fixed verdict `Rejected(Isolation)`. Never sign an
/// `Executed` outcome for code that did not actually run: `complete` would
/// grade it against the hidden answers exactly as if it had.
async fn run_in_isolated_worker(input: &WorkerInput) -> Result<IsolatedAttempt, LabError> {
    // What a real worker would return, as bytes, over its bounded channel.
    let reply = serde_json::to_vec(&WorkerOutput {
        binding: input.binding().clone(),
        outcome: WorkerOutcome::Rejected(ExecutionFailure::Isolation),
    })
    .map_err(|_| LabError::Protocol)?;
    // The controller treats worker bytes as untrusted: bounded decode, then an
    // exact binding check so a reply for another attempt is never signed.
    let output = WorkerOutput::from_bytes(&reply)?;
    if output.binding != *input.binding() {
        return Err(LabError::Protocol);
    }
    Ok(IsolatedAttempt {
        output,
        observations: ContentHash::of(b"byo-example: placeholder, no isolation was observed"),
    })
}

/// PLUG-IN POINT: kill/reap everything your runner owns for this attempt and
/// remove its workspace, then verify absence. Return `Err(Uncertain)` if that
/// cannot be confirmed; the caller then signs nothing and retries later.
/// Nothing is ever launched here, so there is nothing to tear down.
fn tear_down_attempt(_nonce: &Reference) -> Result<(), LabError> {
    Ok(())
}

/// Controller startup (and periodic) recovery. Fences leases a previous
/// controller left behind and attests their cleanup. It never releases source
/// and a cleanup receipt can never award a grade.
async fn recover(store: &SqliteLabs, signer: &ReceiptSigner) -> Result<usize, LabError> {
    let pending = store.cleanup_candidates(32).await?;
    for job in &pending {
        reconcile(store, signer, job).await?;
    }
    Ok(pending.len())
}

/// Claims at most one job and drives it to a terminal or reconciled state.
async fn process_one(
    store: &SqliteLabs,
    signer: &ReceiptSigner,
) -> Result<Option<JobView>, LabError> {
    let Some(job) = store.claim_next().await? else {
        return Ok(None);
    };
    execute_claimed(store, signer, &job).await.map(Some)
}

async fn execute_claimed(
    store: &SqliteLabs,
    signer: &ReceiptSigner,
    job: &LeasedJob,
) -> Result<JobView, LabError> {
    // Sample after `claim_next` returns, on a clock synchronized with the
    // store's: `complete` refuses a receipt that starts before the claim.
    let started_at = SystemClock.now()?;
    let result = match attempt(store, job).await {
        Ok(attempt) => {
            let signed = signer.sign(ExecutionReceipt {
                output: attempt.output,
                started_at,
                // Must not be later than the store's time on receipt and must
                // precede `job.expires_at()`, the lease expiry.
                finished_at: SystemClock.now()?,
                observation_digest: attempt.observations,
                teardown: Teardown::Confirmed,
            })?;
            store.complete(job.scope(), job.id(), &signed).await
        }
        Err(error) => Err(error),
    };
    match result {
        Ok(view) => Ok(view),
        Err(error) => {
            // Fence first, then confirm teardown and attest it. A failed,
            // cancelled, timed-out or lost attempt never becomes a grade.
            let cleanup = store.abandon_attempt(job).await?;
            reconcile(store, signer, &cleanup).await?;
            Err(error)
        }
    }
}

/// Releases work only under a current lease, enforces the exercise's wall
/// limit and stops as soon as cancellation, withdrawal or expiry is recorded.
async fn attempt(store: &SqliteLabs, job: &LeasedJob) -> Result<IsolatedAttempt, LabError> {
    if store.lease_status(job).await? != LeaseStatus::Active {
        return Err(LabError::Conflict);
    }
    let wall = Duration::from_secs(u64::from(job.input().limits().wall_seconds()));
    let deadline = tokio::time::sleep(wall);
    let worker = run_in_isolated_worker(job.input());
    let mut deadline = std::pin::pin!(deadline);
    let mut worker = std::pin::pin!(worker);
    let mut poll = tokio::time::interval(Duration::from_millis(200));
    loop {
        tokio::select! {
            output = &mut worker => return output,
            _ = &mut deadline => return Err(LabError::Expired),
            // A read error is also a stop signal.
            _ = poll.tick() => if store.lease_status(job).await? != LeaseStatus::Active {
                return Err(LabError::Conflict);
            },
        }
    }
}

/// Attests confirmed teardown of an abandoned attempt. The cleanup receipt
/// always reports `Rejected(WorkerLost)` for that attempt's exact binding.
async fn reconcile(
    store: &SqliteLabs,
    signer: &ReceiptSigner,
    job: &CleanupJob,
) -> Result<JobView, LabError> {
    let started_at = SystemClock.now()?;
    tear_down_attempt(&job.binding.nonce)?;
    let receipt = signer.sign(ExecutionReceipt {
        output: WorkerOutput {
            binding: job.binding.clone(),
            outcome: WorkerOutcome::Rejected(ExecutionFailure::WorkerLost),
        },
        started_at,
        finished_at: SystemClock.now()?,
        observation_digest: ContentHash::of(b"byo-example: confirmed attempt absence"),
        teardown: Teardown::Confirmed,
    })?;
    // `true` would re-queue an `Uncertain` pure-function job once, with a new
    // nonce. This controller never re-executes abandoned work automatically.
    store
        .reconcile_cleanup(&job.scope, &job.id, &receipt, false)
        .await
}

/// Demo-only host policy. A web application derives the actor from its
/// authenticated session and current enrollment, never from a request body.
struct DemoPolicy;
impl Authorization for DemoPolicy {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, LabError> {
        let teacher = actor.as_str() == "teacher";
        if scope != &demo_scope()?
            || !(teacher || actor.as_str() == "alice")
            || (matches!(action, Action::ManageExercises | Action::ManageJobs) && !teacher)
        {
            return Err(LabError::Denied);
        }
        Permission::until(
            SystemClock
                .now()?
                .checked_add(3600)
                .ok_or(LabError::Clock)?,
        )
    }
}

fn demo_scope() -> Result<Scope, LabError> {
    Scope::new("school", "rust")
}

fn random_bytes<const N: usize>() -> Result<Zeroizing<[u8; N]>, LabError> {
    use ring::rand::SecureRandom;
    let mut bytes = Zeroizing::new([0u8; N]);
    ring::rand::SystemRandom::new()
        .fill(&mut bytes[..])
        .map_err(|_| LabError::Configuration)?;
    Ok(bytes)
}

/// Only `LinuxExperimental` accepts signed receipts. Despite its name,
/// rullst-labs verifies neither Linux nor isolation: it binds these values
/// into every job and checks receipts against the pinned public key.
fn profile(signer: &ReceiptSigner) -> Result<ExecutionProfile, LabError> {
    // Placeholders, not file digests: pin SHA-256 digests of the files your
    // runner actually installs and loads.
    let pinned = |what: &str| ContentHash::of(what.as_bytes());
    Ok(ExecutionProfile::LinuxExperimental {
        tools: ToolIdentity {
            runner: pinned("placeholder: runner controller build"),
            compiler: pinned("placeholder: compiler"),
            wasm_toolchain: pinned("placeholder: target standard library"),
            runtime: pinned("placeholder: interpreter or runtime"),
            launcher: pinned("placeholder: sandbox launcher"),
            syscall_policy: pinned("placeholder: syscall policy"),
            filesystem_policy: pinned("placeholder: filesystem policy"),
        },
        receipt_key: signer.public_key()?,
    })
}

/// One application handle and one controller handle on the same store.
struct Demo {
    _dir: tempfile::TempDir,
    app: SqliteLabs,
    controller: SqliteLabs,
    signer: ReceiptSigner,
}
impl Demo {
    async fn start() -> Result<Self, LabError> {
        let dir = tempfile::tempdir().map_err(|_| LabError::Storage)?;
        let path: PathBuf = dir.path().join("labs-jobs.sqlite");
        // Separate random secrets: the content key is shared by the
        // application and controller; only the controller holds the seed.
        let content = random_bytes::<32>()?;
        let signer = ReceiptSigner::from_seed(random_bytes::<32>()?)?;
        let config = StoreConfig::new(Reference::new("byo-demo")?, 16, 4, profile(&signer)?)?;
        let app = SqliteLabs::initialize(
            &path,
            config.clone(),
            ContentKey::new(*content)?,
            SystemClock,
        )
        .await?;
        let exercise = Exercise::new(
            demo_scope()?,
            Reference::new("sum")?,
            Reference::new("v1")?,
            vec![
                GraderCase {
                    id: Reference::new("c1")?,
                    input: [2, 3],
                    expected: 5,
                },
                GraderCase {
                    id: Reference::new("c2")?,
                    input: [-4, 10],
                    expected: 6,
                },
            ],
            ExecutionLimits::new(10, 100_000, 64)?,
        )?;
        app.register_exercise(&DemoPolicy, &Reference::new("teacher")?, &exercise)
            .await?;
        // The controller opens the same store with the identical configuration.
        let controller =
            SqliteLabs::open(&path, config, ContentKey::new(*content)?, SystemClock).await?;
        Ok(Self {
            _dir: dir,
            app,
            controller,
            signer,
        })
    }
    async fn submit(&self) -> Result<JobView, LabError> {
        // Submission IDs are course-wide idempotency keys: use random ones.
        let id = Reference::new(hex::encode(&random_bytes::<16>()?[..]))?;
        let submission = Submission::new(
            id,
            ExerciseRef::new("sum", "v1")?,
            RustSource::new("pub fn solve(a: i64, b: i64) -> i64 { a + b }")?,
            300,
        )?;
        self.app
            .submit(
                &DemoPolicy,
                &Reference::new("alice")?,
                &demo_scope()?,
                submission,
            )
            .await
    }
    async fn status(&self, id: &Reference) -> Result<JobView, LabError> {
        self.app
            .get_job(&DemoPolicy, &Reference::new("alice")?, &demo_scope()?, id)
            .await
    }
    async fn close(self) {
        self.controller.close().await;
        self.app.close().await;
    }
}

async fn run() -> Result<JobView, LabError> {
    let demo = Demo::start().await?;
    let queued = demo.submit().await?;
    recover(&demo.controller, &demo.signer).await?;
    process_one(&demo.controller, &demo.signer)
        .await?
        .ok_or(LabError::NotFound)?;
    let view = demo.status(&queued.id).await?;
    demo.close().await;
    Ok(view)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(view) => {
            println!(
                "{}",
                serde_json::json!({"state": view.state, "result": view.result})
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rullst_labs::sqlite::{JobResult, JobState, ResultEvidence};

    #[tokio::test]
    async fn fixed_verdict_receipt_is_accepted_and_never_graded_as_executed() {
        let view = run().await.unwrap();
        assert_eq!(view.state, JobState::Failed);
        assert!(!view.cleanup_pending);
        assert!(matches!(
            view.result,
            Some(JobResult::Rejected {
                failure: ExecutionFailure::Isolation,
                diagnostics: None,
                evidence: ResultEvidence::Experimental { .. },
            })
        ));
    }

    #[tokio::test]
    async fn cancellation_before_release_is_fenced_and_reconciled() {
        let demo = Demo::start().await.unwrap();
        let queued = demo.submit().await.unwrap();
        let job = demo.controller.claim_next().await.unwrap().unwrap();
        let alice = Reference::new("alice").unwrap();
        demo.app
            .cancel(
                &DemoPolicy,
                &alice,
                &demo_scope().unwrap(),
                &queued.id,
                job.revision(),
            )
            .await
            .unwrap();
        let error = execute_claimed(&demo.controller, &demo.signer, &job)
            .await
            .unwrap_err();
        assert_eq!(error, LabError::Conflict);
        let view = demo.status(&queued.id).await.unwrap();
        assert_eq!(view.state, JobState::Cancelled);
        assert!(!view.cleanup_pending && view.result.is_none());
        assert!(
            process_one(&demo.controller, &demo.signer)
                .await
                .unwrap()
                .is_none()
        );
        demo.close().await;
    }

    #[tokio::test]
    async fn startup_recovery_attests_a_lease_left_by_a_lost_controller() {
        let demo = Demo::start().await.unwrap();
        let queued = demo.submit().await.unwrap();
        // A controller claims and then disappears; the learner cancels.
        let lost = demo.controller.claim_next().await.unwrap().unwrap();
        let alice = Reference::new("alice").unwrap();
        let cancelled = demo
            .app
            .cancel(
                &DemoPolicy,
                &alice,
                &demo_scope().unwrap(),
                &queued.id,
                lost.revision(),
            )
            .await
            .unwrap();
        assert!(cancelled.cleanup_pending);
        assert_eq!(recover(&demo.controller, &demo.signer).await.unwrap(), 1);
        let view = demo.status(&queued.id).await.unwrap();
        assert_eq!(view.state, JobState::Cancelled);
        assert!(!view.cleanup_pending);
        assert_eq!(recover(&demo.controller, &demo.signer).await.unwrap(), 0);
        demo.close().await;
    }
}
