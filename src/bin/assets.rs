use arkham_unfolded_experiment::{
    asset_config::AssetConfig,
    assets::{CollectionReport, PrepareResult, inspect_gathering, prepare_gathering},
};
use std::path::PathBuf;

const USAGE: &str = "\
arkham-assets prepare-gathering --source PATH --reveal-icon PATH --ability-icon PATH --output PATH
arkham-assets check-gathering PATH
arkham-assets check-gathering-launch [--art-root PATH]

Preparation copies only the selected assets into a fresh external directory.
The reveal icon is the pinned Lorc Swap bag PNG; preparation never downloads it.
The ability icon is the owner-supplied inner-spark transparent glyph PNG.
It never changes the source archive or overwrites an existing collection.
Launch checking uses Gathering's saved/default folder and temporary artwork flags.
It writes no files and exits nonzero for an incomplete collection.";

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("prepare-gathering") => {
            let mut source = None;
            let mut output = None;
            let mut reveal_icon = None;
            let mut ability_icon = None;
            while let Some(argument) = args.next() {
                let value = args
                    .next()
                    .ok_or_else(|| format!("{argument} needs a path"))?;
                match argument.as_str() {
                    "--source" if source.is_none() => source = Some(PathBuf::from(value)),
                    "--output" if output.is_none() => output = Some(PathBuf::from(value)),
                    "--reveal-icon" if reveal_icon.is_none() => {
                        reveal_icon = Some(PathBuf::from(value))
                    }
                    "--ability-icon" if ability_icon.is_none() => {
                        ability_icon = Some(PathBuf::from(value))
                    }
                    _ => return Err(format!("Unknown or repeated option: {argument}")),
                }
            }
            let source = source.ok_or("prepare requires --source PATH")?;
            let output = output.ok_or("prepare requires --output PATH")?;
            let reveal_icon = reveal_icon.ok_or("prepare requires --reveal-icon PATH")?;
            let ability_icon = ability_icon.ok_or("prepare requires --ability-icon PATH")?;
            let prepare = prepare_gathering;
            let inspect = inspect_gathering;
            match prepare(&source, &output, &reveal_icon, &ability_icon)
                .map_err(|error| error.to_string())?
            {
                PrepareResult::Created => println!("Prepared assets at {}", output.display()),
                PrepareResult::AlreadyPresent => println!(
                    "Identical collection already present at {}; nothing changed.",
                    output.display()
                ),
            }
            println!("{}", inspect(&output).summary());
            Ok(())
        }
        Some("check-gathering") => {
            let root = PathBuf::from(args.next().ok_or("check requires a collection path")?);
            if args.next().is_some() {
                return Err("check accepts one collection path".into());
            }
            let report = inspect_gathering(&root);
            print_check(report)
        }
        Some("check-gathering-launch") => {
            let mut override_root = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--art-root" => {
                        override_root =
                            Some(PathBuf::from(args.next().ok_or("--art-root needs a path")?));
                    }
                    _ => return Err(format!("Unknown Gathering artwork option: {arg}")),
                }
            }
            let config = AssetConfig::load_gathering()?;
            if let Some(warning) = config.warning() {
                eprintln!("Asset settings: {warning}");
            }
            let root = config.launch_root(override_root)?;
            println!("Artwork root: {}", root.display());
            print_check(inspect_gathering(&root))
        }
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("Unknown command: {other}\n{USAGE}")),
    }
}

fn print_check(report: CollectionReport) -> Result<(), String> {
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

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
