use crate::{
    native,
    process::{self, Completion, Identity, OwnedChild},
};
use anyhow::{Context, Result, bail, ensure};
use clap::Args as ClapArgs;
use std::{
    env, fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(ClapArgs)]
pub struct Args {
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=i32::MAX as i64))]
    pid: u32,
    #[arg(long)]
    executable: PathBuf,
    /// Fresh private directory for perf.data and perf.log.
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=60))]
    seconds: u32,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=10))]
    countdown: u32,
}

fn announce(message: impl std::fmt::Display) -> Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(output, "{message}")?;
    output.flush()?;
    Ok(())
}

fn installed(name: &str) -> Result<PathBuf> {
    let paths = env::var_os("PATH").context("PATH is not set")?;
    for directory in env::split_paths(&paths) {
        let candidate = directory.join(name);
        if fs::metadata(&candidate)
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        {
            return Ok(std::path::absolute(candidate)?);
        }
    }
    bail!("Required command is not installed: {name}")
}

fn successful(child: &mut OwnedChild, seconds: u64, label: &str) -> Result<()> {
    match child.wait(Duration::from_secs(seconds))? {
        Completion::Exited(status) if status.success() => Ok(()),
        Completion::Exited(status) => bail!("{label} exited with status {status}"),
        Completion::TimedOut => bail!("{label} timed out"),
        Completion::Interrupted => bail!("Profiling interrupted. No complete capture is claimed."),
    }
}

fn verify(identity: &Identity, pid: u32, executable: &Path) -> Result<()> {
    identity.verify(pid)?;
    identity.verify_executable(executable)
}

pub fn capture(args: Args) -> Result<u8> {
    let executable = args.executable.canonicalize()?;
    let identity = Identity::read(args.pid)?;
    identity.verify_executable(&executable)?;
    let sudo = installed("sudo")?;
    let timeout = installed("timeout")?;
    let perf = installed("perf")?;
    let sleep = installed("sleep")?;
    let output = native::resolve(&args.output_dir)?;
    native::fresh_directory(&output)?;
    announce(format!(
        "Target: PID {}, {}",
        args.pid,
        executable.display()
    ))?;
    announce(format!(
        "Capture: {} seconds; files will stay in {}",
        args.seconds,
        output.display()
    ))?;
    announce(
        "Interaction: repeatedly tilt/orbit with the right mouse button and zoom with the mouse wheel throughout the recording.",
    )?;
    announce(format!(
        "Authenticate below. Then, when the {}-second countdown starts, switch to the app and begin those camera movements.",
        args.countdown
    ))?;
    announce("Leave this terminal open; return after the recording to read its result.")?;
    let result = (|| -> Result<()> {
        // Foreground, inherited terminal: credentials stay between the owner and sudo.
        let mut authentication = Command::new(&sudo);
        authentication.arg("-v");
        let mut auth = OwnedChild::spawn_foreground(&mut authentication)?;
        successful(&mut auth, 120, "Authentication")?;
        auth.cleanup()?;
        for remaining in (1..=args.countdown).rev() {
            announce(format!(
                "Switch to the app and move the camera: recording in {remaining}…"
            ))?;
            process::pause(Duration::from_secs(1))?;
        }
        verify(&identity, args.pid, &executable)?;
        let data = output.join("perf.data");
        let log = native::new_file(&output.join("perf.log"))?;
        let mut command = Command::new(&sudo);
        // The privileged timeout survives loss of this unprivileged supervisor.
        command
            .arg("-n")
            .arg(timeout)
            .args(["--signal=INT", "--kill-after=3"])
            .arg((args.seconds + 5).to_string())
            .arg(perf)
            .args(["record", "-F", "99", "-g", "-p"])
            .arg(args.pid.to_string())
            .arg("-o")
            .arg(&data)
            .arg("--")
            .arg(sleep)
            .arg(args.seconds.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        announce(format!(
            "RECORDING for {} seconds: keep orbiting and zooming.",
            args.seconds
        ))?;
        let mut recording = OwnedChild::spawn(&mut command)?;
        successful(&mut recording, u64::from(args.seconds + 12), "perf")?;
        recording.cleanup()?;
        verify(&identity, args.pid, &executable)?;
        let size = fs::metadata(&data)
            .context("perf produced no capture; inspect perf.log")?
            .len();
        ensure!(size > 0, "perf produced an empty capture; inspect perf.log");
        process::check_interrupt()?;
        announce(format!(
            "Capture complete: {} ({size} bytes). You can stop moving the camera.",
            data.display()
        ))?;
        announce(format!(
            "Profiler output: {}",
            output.join("perf.log").display()
        ))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = announce(format!(
            "Capture did not complete successfully. Any partial files remain in {}.",
            output.display()
        ));
    }
    result?;
    Ok(0)
}
