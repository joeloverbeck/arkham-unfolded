use crate::process::{Completion, Identity, OwnedChild};
mod approval;
mod restart;
use crate::{
    desktop::X11,
    interaction::{self, Desktop},
    portal::DesktopPortal,
};
use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{BufRead, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, PermissionsExt},
        process::ExitStatusExt,
    },
    path::{Component, Path, PathBuf},
    process::{Command as Process, Stdio},
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub enum Command {
    /// Prepare, inspect or close a desktop approval without touching the desktop.
    Approval {
        #[command(subcommand)]
        command: approval::Command,
    },
    /// Stage an immutable executable and supervise an isolated run.
    Launch(Launch),
    /// Restart an isolated run under its original approval and deadline.
    Restart(restart::Restart),
    /// Build an instrumented source copy with isolated Cargo package artifacts.
    ///
    /// Uses offline, locked Cargo with fresh external target and intermediate
    /// build directories so diagnostic libraries cannot enter later normal builds.
    /// Preserves the normal executable.
    Build(Build),
    /// Focus the verified client, then capture its X11 buffer independently of the app.
    Capture(Capture),
    /// One bounded pointer-only desktop portal session, controlled by JSON lines.
    Session(Session),
}

#[derive(Args)]
pub struct Client {
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64))]
    pid: u32,
    #[arg(long, value_parser = window_id)]
    window: u32,
}

fn window_id(value: &str) -> std::result::Result<u32, String> {
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)
    } else {
        value.parse()
    };
    parsed.map_err(|e| e.to_string()).and_then(|id| {
        if id > 0 {
            Ok(id)
        } else {
            Err("Window must be positive".into())
        }
    })
}

#[derive(Args)]
pub struct Capture {
    #[arg(long)]
    approval: Option<String>,
    #[command(flatten)]
    client: Client,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Args)]
#[command(
    after_long_help = r#"JSON actions (one per line; wait for each terminal result):
  click: x, y; optional capture_after: {name, delay_ms} (delay defaults to 0; max 12000)
  scroll: x, y, direction (up or down), steps (positive, 1..=10)
  wheel: legacy signed steps (positive down, negative up, magnitude 1..=10)
  right-drag: x, y, dx, dy, duration_seconds (default 0.6; range 0.2..=2)
  capture: name (fresh .png filename)
  focus, calibrate, stop

Coordinates are client pixels from an inspected capture. Input requires the
verified client, focus, calibrated mapping and no held buttons or modifiers.
Add "focus":true to reacquire focus before a guarded operation.

Example after verifying the target coordinates:
  {"action":"scroll","focus":true,"x":400,"y":300,"direction":"down","steps":1}
  {"action":"capture","focus":true,"name":"after-wheel.png"}
  {"action":"click","focus":true,"x":400,"y":300,"capture_after":{"name":"after-click.png","delay_ms":250}}
  {"action":"stop"}

A click with capture returns clicked with a nested capture result. A capture
failure returns refused with input.status=clicked: inspect or capture again,
never retry that click. The delay and capture share the original command budget.
Help opens no desktop session. Actual input requires the accepted control
interval and pointer-only portal consent. Each command needs 15 seconds before
the final 15-second cleanup reserve. Acknowledgements never renew that budget."#
)]
pub struct Session {
    #[arg(long)]
    approval: Option<String>,
    #[command(flatten)]
    client: Client,
    #[arg(long)]
    output_dir: PathBuf,
    /// Total session limit including desktop consent and idle input.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u32).range(1..=600))]
    timeout_seconds: u32,
    /// Maximum consent/startup wait, bounded by the original session deadline.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=120))]
    consent_timeout_seconds: u32,
    /// Optional earlier end time; cannot extend the approval's original deadline.
    #[arg(long)]
    deadline_unix_seconds: Option<u64>,
}

#[derive(Args)]
pub struct Build {
    #[arg(long, default_value_os_t = repository())]
    source: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value_os_t = default_binary())]
    reference: PathBuf,
    #[arg(long, default_value_t = 1800, value_parser = clap::value_parser!(u32).range(1..=3600))]
    timeout_seconds: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Entry {
    Selection,
    Gathering,
}

#[derive(Clone, Copy, ValueEnum)]
enum LaunchOutput {
    Json,
    Summary,
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn default_binary() -> PathBuf {
    repository().join("target/debug/arkham-tabletop")
}

#[derive(Args)]
pub struct Launch {
    #[arg(long)]
    approval: Option<String>,
    #[arg(long, default_value_os_t = default_binary())]
    binary: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, alias = "experiment", value_enum, default_value_t = Entry::Selection)]
    entry: Entry,
    #[arg(long)]
    load: Option<PathBuf>,
    #[arg(long)]
    art_root: Option<PathBuf>,
    /// Plain filename inside --output-dir, not a path (for example: test.png).
    /// Use --screenshot test.png --quit-after-screenshot for an automatic capture.
    #[arg(long, value_name = "FILENAME")]
    screenshot: Option<String>,
    #[arg(long, requires = "screenshot")]
    quit_after_screenshot: bool,
    /// Diagnostic startup framing; resolve requires Gathering's active drawer.
    /// Leaves the app's Home behavior unchanged.
    #[arg(long, value_parser = ["overview", "scenario", "test", "tony", "patrice", "jacqueline", "resolve"])]
    initial_view: Option<String>,
    /// App lifetime, capped by the approval minus cleanup time. For interactive
    /// review, omit this to use the approval budget; short values suit captures.
    #[arg(long, default_value_t = 1800.0)]
    timeout_seconds: f64,
    /// Stdout events only; run.json always retains the complete manifest.
    #[arg(long, value_enum, default_value_t = LaunchOutput::Json)]
    output_format: LaunchOutput,
}

pub fn run(command: Command) -> Result<u8> {
    match command {
        Command::Approval { command } => approval::run(command),
        Command::Launch(args) => launch(args),
        Command::Restart(args) => restart::run(args),
        Command::Build(args) => build(args),
        Command::Capture(args) => {
            let approval = approval::Approval::admit(args.approval.as_deref())?;
            ensure!(
                approval.remaining()? >= Duration::from_secs(30),
                "Insufficient desktop-control time for capture"
            );
            ensure!(
                fs::symlink_metadata(&args.output).is_err(),
                "Capture already exists; choose a new filename"
            );
            let mut desktop = approval::Desktop {
                approval: &approval,
                inner: X11::new(args.client.pid, args.client.window)?,
            };
            approval.check()?;
            desktop.focus()?;
            let output = std::path::absolute(&args.output)?;
            fs::create_dir_all(
                output
                    .parent()
                    .context("Capture needs a parent directory")?,
            )?;
            approval.check()?;
            emit(&desktop.capture(&output)?)?;
            Ok(0)
        }
        Command::Session(args) => session(args),
    }
}

fn session(args: Session) -> Result<u8> {
    let now = Instant::now();
    let wall_now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let mut remaining = Duration::from_secs(args.timeout_seconds.into());
    if let Some(end) = args.deadline_unix_seconds {
        remaining = remaining.min(
            Duration::from_secs(end)
                .checked_sub(wall_now)
                .filter(|duration| !duration.is_zero())
                .context("Desktop-control deadline has expired")?,
        );
    }
    let deadline = now + remaining;
    let timing = interaction::SessionTiming {
        deadline,
        command_timeout: Duration::from_secs(15),
        now: &Instant::now,
    };
    timing.admit()?;
    let approval = Arc::new(approval::Approval::admit(args.approval.as_deref())?);
    let deadline = deadline.min(Instant::now() + approval.remaining()?);
    let timing = interaction::SessionTiming { deadline, ..timing };
    timing.admit()?;
    let input_deadline = timing.input_deadline();
    let mut desktop = approval::Desktop {
        approval: &approval,
        inner: X11::new(args.client.pid, args.client.window)?,
    };
    let directory = resolve(&args.output_dir)?;
    fresh_directory(&directory)?;
    let mut log = new_file(&directory.join("events.jsonl"))?;
    let (receive, input) = input_lines(std::io::BufReader::new(std::io::stdin()));
    let active_approval = approval.clone();
    input.set_guard(move || active_approval.check())?;
    let mut portal = DesktopPortal::connect(
        input_deadline,
        input.clone(),
        Duration::from_secs(args.consent_timeout_seconds.into()),
    )?;
    let check = || -> Result<()> {
        approval.check()?;
        crate::process::check_interrupt()?;
        input.check()?;
        ensure!(
            Instant::now() < input_deadline,
            "Native verification input deadline reached; closing the session"
        );
        Ok(())
    };
    let mut commands = std::iter::from_fn(|| {
        loop {
            if let Err(error) = check() {
                return Some(Err(error));
            }
            match receive.recv_timeout(Duration::from_millis(50)) {
                Ok(line) => return Some(line.map_err(Into::into)),
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    });
    let mut event = |mut value: Value| -> Result<()> {
        value["time"] = json!(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64());
        value["pid"] = json!(args.client.pid);
        value["window"] = json!(args.client.window);
        serde_json::to_writer(&mut log, &value)?;
        writeln!(log)?;
        log.flush()?;
        emit(&value)
    };
    event(
        json!({"status":"session-budget", "approval":args.approval, "approval_deadline_unix_seconds":approval.deadline_unix_seconds,
        "deadline_unix_seconds":(wall_now + remaining).as_secs_f64().min(approval.deadline_unix_seconds),
        "cleanup_reserve_seconds":interaction::CLEANUP_RESERVE.as_secs(),
        "remaining_ms":deadline.saturating_duration_since(Instant::now()).as_millis(),
        "consent_timeout_seconds":args.consent_timeout_seconds}),
    )?;
    interaction::run_session(
        &mut desktop,
        &mut portal,
        &directory,
        &mut commands,
        &mut event,
        &mut |duration| {
            check()?;
            crate::process::pause(duration)
        },
        timing,
    )?;
    Ok(0)
}

pub(crate) fn input_lines(
    reader: impl BufRead + Send + 'static,
) -> (
    mpsc::Receiver<std::io::Result<String>>,
    Arc<interaction::CommandInput>,
) {
    let (send, receive) = mpsc::sync_channel(8);
    let ended = Arc::new(interaction::CommandInput::default());
    let signal = ended.clone();
    thread::spawn(move || {
        for line in reader.lines() {
            let failed = line.is_err();
            if let Ok(line) = &line {
                signal.observe(line);
            }
            // Do not block the reader behind a pending consent dialog: loss of
            // input (or an overfull command queue) must cancel that request.
            if send.try_send(line).is_err() || failed {
                break;
            }
        }
        signal.end();
    });
    (receive, ended)
}

fn build(args: Build) -> Result<u8> {
    let source = args.source.canonicalize()?;
    let directory = resolve(&args.output_dir)?;
    let reference = args.reference.canonicalize()?;
    ensure!(
        !directory.starts_with(&source) && !directory.starts_with(repository()),
        "Diagnostic output must be outside both source checkouts"
    );
    ensure!(
        !reference.starts_with(&directory),
        "Use a fresh diagnostic directory separate from the preserved executable"
    );
    ensure!(
        source.join("Cargo.toml").is_file(),
        "Source has no Cargo.toml"
    );
    let before = digest(&reference)?;
    fresh_directory(&directory)?;
    let target = directory.join("target");
    let log_path = directory.join("build.log");
    let result = (|| -> Result<()> {
        let log = new_file(&log_path)?;
        let mut command = Process::new("cargo");
        command
            .args(["build", "--locked", "--offline", "--manifest-path"])
            .arg(source.join("Cargo.toml"))
            .args([
                "--features",
                "tabletop",
                "--bin",
                "arkham-tabletop",
                "--target-dir",
            ])
            .arg(&target)
            .env("CARGO_BUILD_BUILD_DIR", target.join("build"))
            .current_dir(&source)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        let mut child = OwnedChild::spawn(&mut command)?;
        match child.wait(Duration::from_secs(args.timeout_seconds.into()))? {
            Completion::Exited(status) if status.success() => Ok(()),
            Completion::Exited(status) => bail!(
                "Diagnostic build failed ({status}); inspect {}",
                log_path.display()
            ),
            Completion::TimedOut => bail!(
                "Diagnostic build timed out; partial evidence remains in {}",
                directory.display()
            ),
            Completion::Interrupted => bail!("Diagnostic build interrupted"),
        }
    })();
    // This preservation check also runs after spawn, compiler or supervision failure.
    ensure!(
        digest(&reference)? == before,
        "Preserved executable changed during the diagnostic build"
    );
    result?;
    let binary = target.join("debug/arkham-tabletop");
    emit(
        &json!({"binary": binary, "sha256": digest(&binary)?, "reference_sha256": before, "log": log_path}),
    )?;
    Ok(0)
}

pub fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("Cannot read {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Resolve the existing ancestors too, so symlinks cannot disguise checkout output.
pub fn resolve(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    let mut ancestor = normalized.as_path();
    let mut missing = Vec::new();
    while fs::symlink_metadata(ancestor).is_err() {
        missing.push(ancestor.file_name().context("Cannot resolve output path")?);
        ancestor = ancestor.parent().context("Cannot resolve output parent")?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

pub fn fresh_directory(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path).is_ok() {
        bail!("Use a fresh diagnostic directory; existing runs are preserved");
    }
    fs::create_dir_all(path.parent().context("Output directory needs a parent")?)?;
    fs::DirBuilder::new().mode(0o700).create(path)?;
    Ok(())
}

pub fn capture_path(directory: &Path, name: &str) -> Result<PathBuf> {
    let filename = Path::new(name);
    ensure!(
        filename.file_name() == Some(filename.as_os_str())
            && filename.extension().is_some_and(|ext| ext == "png"),
        "Use a plain PNG filename inside this run's evidence directory"
    );
    let path = directory.join(name);
    ensure!(
        fs::symlink_metadata(&path).is_err(),
        "Capture already exists; choose a new filename"
    );
    Ok(path)
}

pub fn new_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}

fn copy_verified(source: &Path, destination: &Path) -> Result<String> {
    let bytes = fs::metadata(source)?.len();
    let before = digest(source)?;
    // Only publish a complete verified copy. Dropping this owned temporary file
    // removes partial bytes after quota, space, read or verification failures.
    let mut staged = tempfile::NamedTempFile::new_in(
        destination
            .parent()
            .context("Copy destination needs a parent")?,
    )
    .with_context(|| {
        format!(
            "Cannot prepare staging of {bytes} bytes to {}",
            destination.display()
        )
    })?;
    let copied = (|| -> Result<()> {
        std::io::copy(&mut File::open(source)?, &mut staged)?;
        ensure!(
            digest(staged.path())? == before && digest(source)? == before,
            "Selected source changed while copying; retry in a fresh directory"
        );
        Ok(())
    })();
    if let Err(error) = copied {
        let partial = staged.path().to_owned();
        let cleanup = match staged.close() {
            Ok(()) => "incomplete copy removed".to_owned(),
            Err(error) => format!(
                "could not remove incomplete copy {}: {error}",
                partial.display()
            ),
        };
        return Err(error.context(format!(
            "Cannot stage {bytes} bytes from {} to {}; {cleanup}; choose a destination with sufficient space and quota",
            source.display(), destination.display()
        )));
    }
    staged
        .persist_noclobber(destination)
        .map_err(|error| error.error)
        .with_context(|| format!("Cannot install verified copy at {}", destination.display()))?;
    Ok(before)
}

fn record(directory: &Path, metadata: &Value) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(&mut file, metadata)?;
    writeln!(file)?;
    file.persist(directory.join("run.json"))?;
    Ok(())
}

pub fn emit(value: &Value) -> Result<()> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn emit_launch_summary(directory: &Path, metadata: &Value, exit_code: Option<u8>) -> Result<()> {
    emit(&json!({
        "status": metadata["status"],
        "exit_code": exit_code,
        "pid": metadata["pid"],
        "start_ticks": metadata["start_ticks"],
        "executable": metadata["executable"],
        "executable_sha256": metadata["executable_sha256"],
        "run": directory.join("run.json"),
        "log": metadata["log"],
        "save": metadata["save"],
        "screenshot": metadata["launch_conditions"]["screenshot"],
        "launch_purpose": metadata["launch_purpose"],
        "timing": metadata["timing"],
    }))
}

fn environment_value(value: Option<&OsStr>) -> Value {
    match value {
        None => Value::Null,
        Some(value) => match value.to_str() {
            Some(text) => json!(text),
            None => json!({"unix_bytes": value.as_bytes()}),
        },
    }
}

fn launch_environment(overrides: &std::collections::BTreeMap<&str, PathBuf>) -> Value {
    let mut environment = serde_json::Map::new();
    // Capture only settings relevant to the native graphics comparison, never a
    // complete process environment. Null and empty values remain distinct.
    for name in [
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "WGPU_BACKEND",
        "WGPU_POWER_PREF",
        "WGPU_ADAPTER_NAME",
        "WGPU_FORCE_FALLBACK_ADAPTER",
        "VK_DRIVER_FILES",
        "VK_ADD_DRIVER_FILES",
        "VK_ICD_FILENAMES",
        "VK_INSTANCE_LAYERS",
        "VK_LAYER_PATH",
        "VK_ADD_LAYER_PATH",
        "__GL_SHADER_DISK_CACHE",
        "__GL_SHADER_DISK_CACHE_PATH",
        "__GL_SHADER_DISK_CACHE_SIZE",
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "RUST_LOG",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_SESSION_TYPE",
    ] {
        let inherited = std::env::var_os(name);
        let effective = overrides
            .get(name)
            .map(|path| path.as_os_str())
            .or(inherited.as_deref());
        environment.insert(
            name.to_owned(),
            json!({
                "inherited": environment_value(inherited.as_deref()),
                "effective": environment_value(effective),
            }),
        );
    }
    Value::Object(environment)
}

fn isolated_directories(directory: &Path) -> std::collections::BTreeMap<&str, PathBuf> {
    [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
    ]
    .into_iter()
    .map(|(name, child)| (name, directory.join(child)))
    .collect()
}

fn launch(args: Launch) -> Result<u8> {
    let approval = approval::Approval::admit(args.approval.as_deref())?;
    ensure!(
        (args.initial_view.is_none() && args.load.is_none()) || args.entry == Entry::Gathering,
        "--load and --initial-view require --entry gathering"
    );
    let directory = resolve(&args.output_dir)?;
    let binary = args.binary.canonicalize()?;
    let source_save = args.load.map(fs::canonicalize).transpose()?;
    ensure!(
        !directory.starts_with(repository()),
        "Diagnostic run must be outside the source checkout"
    );
    ensure!(
        binary.is_file() && fs::metadata(&binary)?.permissions().mode() & 0o111 != 0,
        "Selected binary must be an executable file"
    );
    ensure!(
        source_save.as_ref().is_none_or(|path| path.is_file()),
        "Selected save must be a file"
    );
    ensure!(
        args.timeout_seconds.is_finite()
            && args.timeout_seconds > 0.0
            && args.timeout_seconds <= 3600.0,
        "Launch timeout must be greater than zero and at most 3600 seconds"
    );
    let screenshot = args
        .screenshot
        .as_ref()
        .map(|name| capture_path(&directory, name))
        .transpose()?;
    fresh_directory(&directory)?;
    let staged = directory.join("arkham-tabletop");
    let hash = copy_verified(&binary, &staged)?;
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o555))?;
    let save = directory.join("session.json");
    let save_hash = source_save
        .as_ref()
        .map(|source| copy_verified(source, &save))
        .transpose()?;
    let isolated = isolated_directories(&directory);
    for path in isolated.values() {
        fs::create_dir(path)?;
    }
    let mut command = Process::new(&staged);
    match args.entry {
        Entry::Selection => {
            command.args(["--entry", "selection"]);
        }
        Entry::Gathering => {
            command
                .args(["--experiment", "gathering", "--save"])
                .arg(&save);
        }
    }
    if let Some(root) = args.art_root {
        command.arg("--art-root").arg(resolve(&root)?);
    }
    if let Some(path) = &screenshot {
        command.arg("--screenshot").arg(path);
    }
    if args.quit_after_screenshot {
        command.arg("--quit-after-screenshot");
    }
    if let Some(view) = &args.initial_view {
        command.args(["--initial-view", view]);
    }
    let argv: Vec<_> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let mut metadata = json!({"status":"staged", "source_binary":binary, "binary":staged, "sha256":hash,
    "entry_target": match args.entry { Entry::Selection => "selection", Entry::Gathering => "gathering" },
    "argv":argv, "cwd":directory, "environment":isolated, "save":save, "source_save":source_save,
    "source_save_sha256":save_hash, "log":directory.join("app.log"), "timeout_seconds":args.timeout_seconds,
    "initial_view":args.initial_view,
    "launch_conditions": {
        "environment_scope": "allowlist",
        "environment": launch_environment(&isolated),
        "supervisor_cwd": environment_value(Some(std::env::current_dir()?.as_os_str())),
        "executable_staging": "verified-read-only-copy",
        "save_staging": if args.entry == Entry::Selection { "none" } else if source_save.is_some() { "verified-copy" } else { "fresh" },
        "stdin": "null",
        "stdout": directory.join("app.log"),
        "stderr": directory.join("app.log"),
        "screenshot": screenshot,
        "quit_after_screenshot": args.quit_after_screenshot,
        "verify_executable_hash_after_spawn": true
    }});
    metadata["approval"] = json!(args.approval);
    metadata["approval_deadline_unix_seconds"] = json!(approval.deadline_unix_seconds);
    command.current_dir(&directory).envs(&isolated);
    supervise(
        &directory,
        command,
        metadata,
        approval,
        Duration::from_secs_f64(args.timeout_seconds),
        args.output_format,
    )
}

fn supervise(
    directory: &Path,
    mut command: Process,
    mut metadata: Value,
    approval: approval::Approval,
    timeout: Duration,
    output_format: LaunchOutput,
) -> Result<u8> {
    let staged = PathBuf::from(command.get_program());
    let save = PathBuf::from(
        metadata["save"]
            .as_str()
            .context("Missing isolated save path")?,
    );
    metadata["save_existed_at_start"] = json!(save.try_exists().ok());
    record(directory, &metadata)?;
    let log = new_file(&directory.join("app.log"))?;
    command
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    approval.check()?;
    let runtime = approval
        .remaining()?
        .checked_sub(interaction::CLEANUP_RESERVE)
        .context("Insufficient desktop-control time to launch and clean up")?;
    let started = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
    let approval_end = approval.deadline_unix_seconds - interaction::CLEANUP_RESERVE.as_secs_f64();
    let shutdown = (started + timeout.min(runtime).as_secs_f64()).min(approval_end);
    ensure!(
        shutdown > started,
        "Insufficient desktop-control time to launch and clean up"
    );
    let deadline = Instant::now() + Duration::from_secs_f64(shutdown - started);
    metadata["launch_purpose"] = json!(if metadata["launch_conditions"]["quit_after_screenshot"]
        == true
    {
        "automatic_capture"
    } else {
        "interactive"
    });
    metadata["timing"] = json!({
        "started_unix_seconds": started,
        "shutdown_unix_seconds": shutdown,
        "approval_deadline_unix_seconds": approval.deadline_unix_seconds,
        "cleanup_reserve_seconds": interaction::CLEANUP_RESERVE.as_secs_f64(),
        "app_timeout_seconds": timeout.as_secs_f64(),
        "runtime_budget_seconds": shutdown - started,
        "limiting_condition": if timeout < runtime && started + timeout.as_secs_f64() < approval_end {
            "app_timeout"
        } else {
            "approval_cleanup_reserve"
        }
    });
    record(directory, &metadata)?;
    let mut child = OwnedChild::spawn(&mut command)?;
    metadata["pid"] = json!(child.id());
    let outcome = (|| -> Result<u8> {
        let identity = Identity::read(child.id())?;
        identity.verify_executable(&staged)?;
        metadata["status"] = json!("running");
        metadata["start_ticks"] = json!(identity.start_ticks);
        metadata["executable"] = json!(identity.executable);
        metadata["executable_sha256"] =
            json!(digest(&PathBuf::from(format!("/proc/{}/exe", child.id())))?);
        ensure!(
            metadata["executable_sha256"] == metadata["sha256"],
            "Staged executable changed after verification"
        );
        record(directory, &metadata)?;
        match output_format {
            LaunchOutput::Json => emit(&metadata)?,
            LaunchOutput::Summary => emit_launch_summary(directory, &metadata, None)?,
        }
        // Identity verification and output consume the same lifetime; neither
        // may move the advertised shutdown beyond the original approval.
        match child.wait_checked(deadline.saturating_duration_since(Instant::now()), || {
            approval.check()
        })? {
            Completion::Exited(status) => {
                metadata["status"] = json!("exited");
                Ok(status
                    .code()
                    .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)) as u8)
            }
            Completion::TimedOut => {
                metadata["status"] = json!("timed-out");
                Ok(124)
            }
            Completion::Interrupted => bail!("Launch interrupted"),
        }
    })();
    if let Err(error) = &outcome {
        metadata["status"] = json!("interrupted-or-failed");
        metadata["error"] = json!(format!("{error:#}"));
    }
    let cleanup = child.cleanup();
    if let Ok(Some(status)) = &cleanup {
        metadata["returncode"] = json!(
            status
                .code()
                .unwrap_or_else(|| -status.signal().unwrap_or(0))
        );
    }
    metadata["save_exists_at_exit"] = json!(save.try_exists().ok());
    record(directory, &metadata)?;
    let result = cleanup.and(outcome);
    match output_format {
        LaunchOutput::Json => {
            let code = result?;
            emit(&metadata)?;
            Ok(code)
        }
        LaunchOutput::Summary => {
            let code = result.as_ref().copied().unwrap_or_else(|_| {
                if crate::process::interrupted() {
                    130
                } else {
                    1
                }
            });
            emit_launch_summary(directory, &metadata, Some(code))?;
            result
        }
    }
}
