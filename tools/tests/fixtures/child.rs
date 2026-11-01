use std::{env, fs, io::Write, process::{Command, ExitCode}, thread, time::Duration};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--fixture-record-check") {
        println!("RECORD_CHECK_STDOUT");
        eprintln!("RECORD_CHECK_STDERR");
        println!("cwd={}", env::current_dir().unwrap().display());
        println!("env={}", env::var("ARKHAM_FIXTURE_RECORD_ENV").unwrap_or_default());
        if env::var_os("ARKHAM_FIXTURE_DESCENDANT").is_some() {
            let child = Command::new(env::current_exe().unwrap()).arg("--fixture-descendant").spawn().unwrap();
            fs::write("descendant.pid", child.id().to_string()).unwrap();
        }
        println!("RECORD_CHECK_READY");
        std::io::stdout().flush().unwrap();
        std::io::stderr().flush().unwrap();
        if args.get(1).is_some_and(|arg| arg == "wait") {
            loop { thread::sleep(Duration::from_secs(1)); }
        }
        return ExitCode::from(args[1].to_str().unwrap().parse::<u8>().unwrap());
    }
    let program = env::args_os().next().unwrap();
    let name = std::path::Path::new(&program).file_name().unwrap();
    if let Ok(mode) = env::var("ARKHAM_FIXTURE_RUSTUP") {
        if name == "cargo" { eprintln!("CARGO_MUST_NOT_RUN_DURING_BOOTSTRAP"); return ExitCode::from(97); }
        let mut trace = fs::OpenOptions::new().create(true).append(true).open(env::var_os("ARKHAM_FIXTURE_TRACE").unwrap()).unwrap();
        writeln!(trace, "{}", args.iter().map(|arg| arg.to_string_lossy()).collect::<Vec<_>>().join(" ")).unwrap();
        if args == ["show", "active-toolchain"] {
            match mode.as_str() {
                "show-fail" => return ExitCode::from(1),
                "empty" => {},
                "stable" => println!("stable-x86_64-unknown-linux-gnu (default)"),
                _ => println!("1.97.1-x86_64-unknown-linux-gnu (overridden by rust-toolchain.toml)"),
            }
        }
        return ExitCode::from(if (mode == "default-fail" && args.first().is_some_and(|arg| arg == "default")) || (mode == "uninstall-fail" && args.first().is_some_and(|arg| arg == "toolchain")) { 1 } else { 0 });
    }
    if env::var_os("ARKHAM_FIXTURE_CHECK").is_some() {
        if args.iter().any(|arg| arg == "vendor/wgpu-hal/Cargo.toml") {
            assert_eq!(env::var_os("CARGO_TARGET_DIR").unwrap(), env::current_dir().unwrap().join("target").into_os_string());
        }
        let trace = env::var_os("ARKHAM_FIXTURE_TRACE").unwrap();
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&trace).unwrap();
        writeln!(file, "{}", args.iter().map(|arg| arg.to_string_lossy()).collect::<Vec<_>>().join(" ")).unwrap();
        let count = fs::read_to_string(trace).unwrap().lines().count();
        if env::var("ARKHAM_FIXTURE_FAIL_CHECK").ok().and_then(|value| value.parse::<usize>().ok()) == Some(count) { return ExitCode::from(42); }
        if args.iter().any(|arg| arg == "check-tracked-files") {
            return ExitCode::from(Command::new(env::var_os("ARKHAM_FIXTURE_TOOLS").unwrap()).arg("check-tracked-files").status().unwrap().code().unwrap_or(1) as u8);
        }
        return ExitCode::SUCCESS;
    }
    if let Ok(mode) = env::var("ARKHAM_FIXTURE_PROFILE") {
        if name == "sudo" {
            if args == ["-v"] {
                println!("FIXTURE_AUTH");
                if mode == "auth-fail" { return ExitCode::from(1); }
                if mode == "replace-before" {
                    let binary = env::var_os("ARKHAM_FIXTURE_TARGET_BINARY").unwrap();
                    let replacement = std::path::Path::new(&binary).with_extension("replacement");
                    fs::copy(&binary, &replacement).unwrap();
                    fs::rename(replacement, binary).unwrap();
                }
                return ExitCode::SUCCESS;
            }
            fs::write(env::var_os("ARKHAM_FIXTURE_TRACE").unwrap(), args.iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join("\n")).unwrap();
            assert_eq!(args[0], "-n");
            return ExitCode::from(Command::new(&args[1]).args(&args[2..]).status().unwrap().code().unwrap_or(1) as u8);
        }
        if name == "timeout" {
            assert_eq!(args[0], "--signal=INT");
            assert_eq!(args[1], "--kill-after=3");
            return ExitCode::from(Command::new(&args[3]).args(&args[4..]).status().unwrap().code().unwrap_or(1) as u8);
        }
        if name == "perf" {
            let output = &args[args.iter().position(|arg| arg == "-o").unwrap() + 1];
            if mode != "missing" { fs::write(output, if mode == "empty" { "" } else { "captured samples" }).unwrap(); }
            println!("perf statistics");
            if mode == "replace-after" {
                let binary = env::var_os("ARKHAM_FIXTURE_TARGET_BINARY").unwrap();
                let replacement = std::path::Path::new(&binary).with_extension("replacement");
                fs::copy(&binary, &replacement).unwrap();
                fs::rename(replacement, binary).unwrap();
            }
            if mode == "hang" { loop { thread::sleep(Duration::from_secs(1)); } }
            return ExitCode::from(if mode == "fail" { 124 } else { 0 });
        }
    }
    if env::var_os("ARKHAM_FIXTURE_IGNORE_TERM").is_some() {
        unsafe extern "C" { fn signal(number: i32, handler: usize) -> usize; }
        // SAFETY: the Linux test child deliberately ignores SIGTERM to exercise escalation.
        unsafe { signal(15, 1); }
    }
    if args.iter().any(|arg| arg == "--fixture-descendant") {
        loop { thread::sleep(Duration::from_secs(1)); }
    }
    if let Some(tool) = env::var_os("ARKHAM_FIXTURE_TOOLS") {
        if args.first().is_some_and(|arg| arg == "run") {
            let separator = args.iter().position(|arg| arg == "--").unwrap();
            return ExitCode::from(Command::new(tool).args(&args[separator + 1..]).status().unwrap().code().unwrap_or(1) as u8);
        }
        eprintln!("UNEXPECTED_LATER_CHECK");
        return ExitCode::from(97);
    }
    if args.iter().any(|arg| arg == "--save" || arg == "--load") {
        if env::var_os("ARKHAM_FIXTURE_DESCENDANT").is_some() {
            let child = Command::new(env::current_exe().unwrap()).arg("--fixture-descendant").spawn().unwrap();
            fs::write("descendant.pid", child.id().to_string()).unwrap();
        }
        let position = args.iter().position(|arg| arg == "--save" || arg == "--load").unwrap();
        fs::write(&args[position + 1], "autosaved copy").unwrap();
        for name in ["XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"] {
            println!("{name}={}", env::var(name).unwrap());
        }
        println!("cwd={}", env::current_dir().unwrap().display());
        std::io::stdout().flush().unwrap();
        if env::var_os("ARKHAM_FIXTURE_EXEC_HANDOFF").is_some() {
            use std::os::unix::process::CommandExt;
            thread::sleep(Duration::from_millis(150));
            panic!("exec sleep failed: {}", Command::new("sleep").arg("60").exec());
        }
        while !std::path::Path::new("finish").exists() {
            thread::sleep(Duration::from_millis(10));
        }
        println!("original executable");
        return ExitCode::from(env::var("ARKHAM_FIXTURE_EXIT_CODE").ok().map(|code| code.parse::<u8>().unwrap()).unwrap_or(0));
    }
    ExitCode::from(98)
}
