use std::{env, ffi::OsString, fs, io::Write, path::PathBuf, process::ExitCode};

// Script only the external gh process. Every unexpected invocation fails, and
// the journal exposes whether a rejected prerequisite still reached a merge.
pub fn run(args: &[OsString]) -> ExitCode {
    let root = PathBuf::from(env::var_os("ARKHAM_FIXTURE_GITHUB").unwrap());
    let journal = root.join("calls");
    let count = fs::read_to_string(&journal)
        .unwrap_or_default()
        .lines()
        .count();
    let args: Vec<_> = args.iter().map(|arg| arg.to_str().unwrap()).collect();
    let mut trace = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .unwrap();
    writeln!(trace, "{}", args.join("\t")).unwrap();
    // A request body read from standard input is kept for the test to inspect.
    if args.windows(2).any(|pair| pair == ["--input", "-"]) {
        let mut input = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut input).unwrap();
        fs::write(root.join(format!("{count}.input")), input).unwrap();
    }
    let expected = fs::read_to_string(root.join(format!("{count}.args"))).unwrap_or_default();
    if args.join("\n") != expected {
        eprintln!("Unexpected gh invocation {count}: {args:?}; expected {expected:?}");
        return ExitCode::from(97);
    }
    print!(
        "{}",
        fs::read_to_string(root.join(format!("{count}.stdout"))).unwrap()
    );
    let code: u8 = fs::read_to_string(root.join(format!("{count}.status")))
        .unwrap()
        .parse()
        .unwrap();
    if code != 0 {
        eprintln!("fixture gh failure {count}");
    }
    ExitCode::from(code)
}
