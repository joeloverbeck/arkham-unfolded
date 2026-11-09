//! Restart only an isolated native run, under its original desktop approval.
use super::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Args)]
pub struct Restart {
    /// Completed run.json from native launch or restart.
    #[arg(long)]
    run: PathBuf,
    /// Fresh directory for this supervisor's manifest and log.
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, value_enum, default_value_t = LaunchOutput::Json)]
    output_format: LaunchOutput,
}

#[derive(Deserialize)]
struct Previous {
    status: String,
    entry_target: String,
    #[serde(rename = "returncode")]
    _returncode: i32,
    binary: PathBuf,
    sha256: String,
    cwd: PathBuf,
    environment: BTreeMap<String, PathBuf>,
    save: PathBuf,
    save_existed_at_start: Option<bool>,
    save_exists_at_exit: Option<bool>,
    argv: Vec<String>,
    approval: String,
    approval_deadline_unix_seconds: f64,
    timeout_seconds: f64,
    pid: u32,
    start_ticks: String,
}

pub fn run(args: Restart) -> Result<u8> {
    let source = args.run.canonicalize()?;
    let bytes = fs::read(&source)?;
    let metadata: Value = serde_json::from_slice(&bytes)?;
    ensure!(
        metadata["installation"] != "normal",
        "Normal-installation checks require a fresh launch and Continue snapshot"
    );
    let previous: Previous = serde_json::from_slice(&bytes).context("Read completed native run")?;
    ensure!(
        matches!(
            previous.status.as_str(),
            "exited" | "timed-out" | "interrupted-or-failed"
        ),
        "Restart requires a completed supervisor record"
    );
    // Deserializing returncode requires evidence that cleanup reaped the old child.
    ensure!(
        !Path::new(&format!("/proc/{}", previous.pid)).exists()
            || Identity::read(previous.pid)?.start_ticks != previous.start_ticks,
        "Previous native process is still present"
    );
    let approval = approval::Approval::admit(Some(&previous.approval))?;
    ensure!(
        approval.deadline_unix_seconds == previous.approval_deadline_unix_seconds,
        "Original approval deadline changed"
    );
    ensure!(
        previous.timeout_seconds.is_finite()
            && previous.timeout_seconds > 0.0
            && previous.timeout_seconds <= 3600.0,
        "Invalid original launch timeout"
    );
    let cwd = previous.cwd.canonicalize()?;
    ensure!(
        cwd == previous.cwd && !cwd.starts_with(repository()),
        "Restart requires the original isolated working directory"
    );
    ensure!(
        previous.binary == cwd.join("arkham-tabletop")
            && previous.binary.canonicalize()? == previous.binary,
        "Restart requires the original staged executable"
    );
    let permissions = fs::metadata(&previous.binary)?.permissions().mode();
    ensure!(
        digest(&previous.binary)? == previous.sha256
            && permissions & 0o222 == 0
            && permissions & 0o111 != 0,
        "Staged executable changed"
    );
    ensure!(
        previous.save == cwd.join("session.json"),
        "Restart requires the original isolated save path"
    );
    let has_save = match fs::symlink_metadata(&previous.save) {
        Ok(_) => {
            ensure!(
                previous.save.canonicalize()? == previous.save && previous.save.is_file(),
                "Isolated save path changed"
            );
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Folder selection can finish before the player saves any game.
            // Older records without presence evidence cannot justify a missing file.
            ensure!(
                previous.save_existed_at_start == Some(false)
                    && previous.save_exists_at_exit == Some(false),
                "Recorded isolated save is missing"
            );
            false
        }
        Err(error) => return Err(error.into()),
    };
    let expected = isolated_directories(&cwd);
    ensure!(
        previous.environment.len() == expected.len()
            && expected
                .iter()
                .all(|(name, path)| previous.environment.get(*name) == Some(path)),
        "Original isolated settings paths changed"
    );
    for path in expected.values() {
        ensure!(
            path.is_dir() && path.canonicalize()? == *path,
            "Isolated settings directory changed"
        );
    }
    let entry = Entry::from_str(&previous.entry_target, false).map_err(anyhow::Error::msg)?;
    let expected_selector = match entry {
        Entry::Selection => ["--entry", "selection"],
        Entry::Gathering => ["--experiment", "gathering"],
    };
    ensure!(
        previous
            .argv
            .windows(2)
            .any(|pair| pair == expected_selector)
            && previous
                .argv
                .iter()
                .filter(|arg| matches!(arg.split('=').next(), Some("--entry" | "--experiment")))
                .count()
                == 1,
        "Recorded entry target disagrees with invocation"
    );
    // Serialize restarts that share settings/save, even from the same old manifest.
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(cwd.join("restart.lock"))?;
    lock.try_lock()
        .context("Another restart owns these isolated settings and save")?;
    let directory = resolve(&args.output_dir)?;
    ensure!(
        !directory.starts_with(repository()),
        "Diagnostic run must be outside the source checkout"
    );
    approval
        .remaining()?
        .checked_sub(interaction::CLEANUP_RESERVE)
        .context("Insufficient desktop-control time to restart and clean up")?;
    fresh_directory(&directory)?;

    let mut command = Process::new(&previous.binary);
    command.args(expected_selector);
    if entry == Entry::Gathering {
        command.arg("--save").arg(&previous.save);
    }
    command
        .current_dir(&cwd)
        .envs(&expected)
        .env_remove("ARKHAM_VERIFICATION_ATTEMPTS_DIR");
    let argv: Vec<_> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let metadata = json!({
        "status":"staged", "source_binary":previous.binary, "binary":previous.binary, "sha256":previous.sha256,
        "entry_target":previous.entry_target,
        "argv":argv, "cwd":cwd, "environment":expected, "save":previous.save,
        "source_save":null, "source_save_sha256":null, "log":directory.join("app.log"),
        "timeout_seconds":previous.timeout_seconds, "initial_view":null,
        "approval":previous.approval, "approval_deadline_unix_seconds":previous.approval_deadline_unix_seconds,
        "restart_of":{"run":source,"sha256":format!("{:x}", Sha256::digest(&bytes))},
        "launch_conditions": {
            "environment_scope":"allowlist", "environment":launch_environment(&expected),
            "supervisor_cwd":environment_value(Some(std::env::current_dir()?.as_os_str())),
            "executable_staging":"reuse-verified-copy", "save_staging":if has_save { "reuse-existing" } else { "reuse-unsaved-path" },
            "stdin":"null", "stdout":directory.join("app.log"), "stderr":directory.join("app.log"),
            "screenshot":null, "quit_after_screenshot":false, "verify_executable_hash_after_spawn":true
        }
    });
    supervise(
        &directory,
        command,
        metadata,
        approval,
        Duration::from_secs_f64(previous.timeout_seconds),
        args.output_format,
    )
}
