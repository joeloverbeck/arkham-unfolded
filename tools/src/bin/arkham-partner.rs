//! The agent partners' client program. It is a program of its own, so that the permission
//! a partner's session has for it opens nothing else of `arkham-tools`.

#[path = "../partner.rs"]
mod partner;
use arkham_tools::process;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = partner::Args::parse();
    if let Err(error) = process::install_interrupt_handler() {
        eprintln!("arkham-partner: {error}");
        return ExitCode::from(1);
    }
    match partner::run(args) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("arkham-partner: {error:#}");
            ExitCode::from(if process::interrupted() { 130 } else { 2 })
        }
    }
}
