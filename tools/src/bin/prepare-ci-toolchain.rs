// This entry point also compiles directly with rustc before CI restores Cargo's cache.
#[path = "../ci.rs"]
mod ci;

use std::process::ExitCode;

fn main() -> ExitCode {
    match ci::prepare_toolchain() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("prepare-ci-toolchain: {error}");
            ExitCode::from(2)
        }
    }
}
