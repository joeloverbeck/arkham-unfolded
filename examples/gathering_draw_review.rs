//! Prepare isolated ordinary saves; this command never launches a desktop.
use arkham_unfolded_experiment::gathering;
use gathering::continuity::{load_session, save_session};
use serde_json::json;
use std::{error::Error, path::PathBuf};

#[path = "../tests/support/gathering_draw_review.rs"]
mod draw_review;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(
        args.next()
            .ok_or("Usage: gathering_draw_review FRESH_DIRECTORY")?,
    );
    if args.next().is_some() {
        return Err("Expected only a fresh output directory".into());
    }
    let checkpoints = draw_review::checkpoints()?;
    std::fs::create_dir(&output)?;
    let mut records = Vec::new();
    for checkpoint in checkpoints {
        let path = output.join(format!("{}.json", checkpoint.name));
        save_session(&path, &checkpoint.session)?;
        let loaded = load_session(&path)?;
        if loaded.clone() != checkpoint.session.clone() || loaded.is_verification_fixture() {
            return Err(format!("Ordinary save round-trip failed: {}", path.display()).into());
        }
        let observed = loaded.observe();
        let label = &observed
            .choices
            .iter()
            .find(|choice| choice.id == checkpoint.trigger)
            .ok_or("Prepared trigger is unavailable")?
            .label;
        records.push(
            json!({"name":checkpoint.name, "path":path, "revision":observed.revision,
            "initial_view":checkpoint.initial_view, "trigger":checkpoint.trigger, "label":label}),
        );
    }
    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}
