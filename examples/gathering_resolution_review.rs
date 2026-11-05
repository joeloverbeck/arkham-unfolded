//! Export isolated ordinary saves for the bounded native resolution-tray review.
use arkham_unfolded_experiment::gathering;
use gathering::continuity::{load_session, save_session};
use serde_json::json;
use std::{error::Error, path::PathBuf};

#[path = "../tests/support/gathering_resolution_review.rs"]
mod review;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(
        args.next()
            .ok_or("Usage: gathering_resolution_review FRESH_DIRECTORY")?,
    );
    if args.next().is_some() {
        return Err("Expected only a fresh output directory".into());
    }
    let checkpoints = review::checkpoints()?;
    std::fs::create_dir(&output)?;
    let mut records = vec![];
    for checkpoint in checkpoints {
        let path = output.join(format!("{}.json", checkpoint.name));
        save_session(&path, &checkpoint.session)?;
        let loaded = load_session(&path)?;
        if loaded != checkpoint.session || loaded.is_verification_fixture() {
            return Err(format!("Ordinary save round-trip failed: {}", path.display()).into());
        }
        let view = loaded.observe();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id == checkpoint.trigger)
            .ok_or_else(|| format!("{}: prepared trigger is unavailable", checkpoint.name))?;
        records.push(json!({"name":checkpoint.name, "path":path, "revision":view.revision,
            "initial_view":checkpoint.initial_view, "trigger":checkpoint.trigger, "label":choice.label}));
    }
    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}
