use arkham_unfolded_experiment::assets::{
    PrepareResult, inspect, inspect_tony, prepare, prepare_tony,
};
use std::path::PathBuf;

const USAGE: &str = "arkham-assets prepare --source PATH --output PATH\narkham-assets check PATH\narkham-assets prepare-tony --source PATH --output PATH\narkham-assets check-tony PATH\n\nPreparation copies only the selected assets into a fresh external directory.\nIt never changes the source archive or overwrites an existing collection.";

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some(command @ ("prepare" | "prepare-tony")) => {
            let mut source = None;
            let mut output = None;
            while let Some(argument) = args.next() {
                let value = args
                    .next()
                    .ok_or_else(|| format!("{argument} needs a path"))?;
                match argument.as_str() {
                    "--source" if source.is_none() => source = Some(PathBuf::from(value)),
                    "--output" if output.is_none() => output = Some(PathBuf::from(value)),
                    _ => return Err(format!("Unknown or repeated option: {argument}")),
                }
            }
            let source = source.ok_or("prepare requires --source PATH")?;
            let output = output.ok_or("prepare requires --output PATH")?;
            let prepare = if command == "prepare-tony" {
                prepare_tony
            } else {
                prepare
            };
            let inspect = if command == "prepare-tony" {
                inspect_tony
            } else {
                inspect
            };
            match prepare(&source, &output).map_err(|error| error.to_string())? {
                PrepareResult::Created => println!("Prepared assets at {}", output.display()),
                PrepareResult::AlreadyPresent => println!(
                    "Identical collection already present at {}; nothing changed.",
                    output.display()
                ),
            }
            println!("{}", inspect(&output).summary());
            Ok(())
        }
        Some(command @ ("check" | "check-tony")) => {
            let root = PathBuf::from(args.next().ok_or("check requires a collection path")?);
            if args.next().is_some() {
                return Err("check accepts one collection path".into());
            }
            let report = if command == "check-tony" {
                inspect_tony(&root)
            } else {
                inspect(&root)
            };
            println!("{}", report.summary());
            for diagnostic in &report.diagnostics {
                println!("- {diagnostic}");
            }
            if report.complete {
                Ok(())
            } else {
                Err("The collection is not complete for this application.".into())
            }
        }
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("Unknown command: {other}\n{USAGE}")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
