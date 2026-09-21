//! Local application-side fixture/usage example. The local operator supplies the
//! actor through stdin; a web application MUST replace this policy/transport with
//! its authenticated identity and current enrollment authority. No code executes
//! here and the runner is neither a dependency nor a spawned child.
use rullst_labs::{
    sqlite::{ContentKey, SqliteLabs, StoreConfig},
    *,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{BufRead, Read, Write},
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    database: PathBuf,
    namespace: Reference,
    max_jobs: u32,
    max_exercises: u32,
    content_key: PathBuf,
    profile: ExecutionProfile,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    actor: Reference,
    scope: Scope,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Operation {
    Register { exercise: Exercise },
    Submit { submission: Submission },
    Status { id: Reference },
    Cancel { id: Reference, revision: i64 },
    Withdraw { exercise: ExerciseRef },
}
struct LocalOperatorPolicy;
impl Authorization for LocalOperatorPolicy {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, LabError> {
        if scope != &Scope::new("school", "rust")?
            || !matches!(actor.as_str(), "teacher" | "alice" | "bob")
            || (matches!(action, Action::ManageExercises | Action::ManageJobs)
                && actor.as_str() != "teacher")
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
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
async fn run() -> Result<(), LabError> {
    let args: Vec<_> = std::env::args_os()
        .skip(1)
        .take(3)
        .map(|value| value.into_string().map_err(|_| LabError::Configuration))
        .collect::<Result<_, _>>()?;
    let [mode, path] = args.as_slice() else {
        return Err(LabError::Configuration);
    };
    if !matches!(mode.as_str(), "initialize" | "serve") {
        return Err(LabError::Configuration);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(32769).read_to_end(&mut bytes))
        .map_err(|_| LabError::Configuration)?;
    if bytes.len() > 32768 {
        return Err(LabError::Configuration);
    }
    let config: Configuration =
        serde_json::from_slice(&bytes).map_err(|_| LabError::Configuration)?;
    let mut key = zeroize::Zeroizing::new(Vec::new());
    std::fs::File::open(&config.content_key)
        .and_then(|f| f.take(33).read_to_end(&mut key))
        .map_err(|_| LabError::Configuration)?;
    let array: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| LabError::Configuration)?;
    let settings = StoreConfig::new(
        config.namespace,
        config.max_jobs,
        config.max_exercises,
        config.profile,
    )?;
    let store = if mode == "initialize" {
        SqliteLabs::initialize(
            &config.database,
            settings,
            ContentKey::new(array)?,
            SystemClock,
        )
        .await?
    } else {
        SqliteLabs::open(
            &config.database,
            settings,
            ContentKey::new(array)?,
            SystemClock,
        )
        .await?
    };
    if mode == "initialize" {
        store.close().await;
        return Ok(());
    }
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = zeroize::Zeroizing::new(Vec::new());
        let count = input
            .by_ref()
            .take(131073)
            .read_until(b'\n', &mut line)
            .map_err(|_| LabError::Protocol)?;
        if count == 0 {
            break;
        }
        if count > 131072 {
            return Err(LabError::Capacity);
        }
        let result = match serde_json::from_slice::<Request>(&line) {
            Ok(request) => process(&store, request).await,
            Err(_) => Err(LabError::InvalidInput),
        };
        let response = match result {
            Ok(value) => json!({"ok":value}),
            Err(error) => json!({"error":error.to_string()}),
        };
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, &response).map_err(|_| LabError::Protocol)?;
        output
            .write_all(b"\n")
            .and_then(|_| output.flush())
            .map_err(|_| LabError::Protocol)?;
    }
    store.close().await;
    Ok(())
}
async fn process(store: &SqliteLabs, request: Request) -> Result<serde_json::Value, LabError> {
    let Request {
        actor,
        scope,
        operation,
    } = request;
    match operation {
        Operation::Register { exercise } => {
            if exercise.scope() != &scope {
                return Err(LabError::Denied);
            }
            store
                .register_exercise(&LocalOperatorPolicy, &actor, &exercise)
                .await?;
            Ok(json!({"registered":true}))
        }
        Operation::Submit { submission } => Ok(json!(
            store
                .submit(&LocalOperatorPolicy, &actor, &scope, submission)
                .await?
        )),
        Operation::Status { id } => Ok(json!(
            store
                .get_job(&LocalOperatorPolicy, &actor, &scope, &id)
                .await?
        )),
        Operation::Cancel { id, revision } => Ok(json!(
            store
                .cancel(&LocalOperatorPolicy, &actor, &scope, &id, revision)
                .await?
        )),
        Operation::Withdraw { exercise } => {
            store
                .set_exercise_enabled(
                    &LocalOperatorPolicy,
                    &actor,
                    &scope,
                    &exercise.id,
                    &exercise.revision,
                    false,
                )
                .await?;
            Ok(json!({"withdrawn":true}))
        }
    }
}
