use arkham_unfolded_experiment::gathering::Session;
use arkham_unfolded_experiment::gathering::continuity::{
    commit_choice, load_session, save_session,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "arkham-driver --experiment gathering [--load PATH] [--save PATH]\narkham-driver --experiment gathering --inspect PATH\nInspection emits one validated player observation and exits without writing files or reading stdin; it cannot be combined with --load or --save. Live mode saves on startup. JSON lines: observe; choose with revision and id; save; reopen; quit. A listed choice id ending in :<margin> names a number: choose it with a whole number from 0 to 65535 in place of <margin> (Crystal Pendulum predictions). A listed choice id ending in :<contributions> is a group clue payment: choose it with the clues each investigator spends, in the order Tony, Patrice, Jacqueline, separated by colons (act:pay:2:4:0); the observation's group_clue_payment gives eligibility, holdings, the requirement and any forced split. An existing save reopens automatically. Inputs use the pinned research baseline, not a current campaign update check. Random draws are session-owned and not player-selectable.";

enum Options {
    Inspect(PathBuf),
    Live {
        load: Option<PathBuf>,
        save: PathBuf,
    },
}
impl Options {
    fn parse() -> Result<Option<Self>, String> {
        let mut args = std::env::args().skip(1);
        let mut load = None;
        let mut save = None;
        let mut inspect = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => return Ok(None),
                "--experiment" => {
                    if args.next().as_deref() != Some("gathering") {
                        return Err("--experiment must be gathering".into());
                    }
                }
                "--load" => load = Some(PathBuf::from(args.next().ok_or("--load needs a path")?)),
                "--save" => save = Some(PathBuf::from(args.next().ok_or("--save needs a path")?)),
                "--inspect" => {
                    inspect = Some(PathBuf::from(args.next().ok_or("--inspect needs a path")?));
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if let Some(path) = inspect {
            if load.is_some() || save.is_some() {
                return Err("--inspect cannot be combined with --load or --save".into());
            }
            return Ok(Some(Self::Inspect(path)));
        }
        let save = save
            .or_else(|| load.clone())
            .unwrap_or_else(|| PathBuf::from("local/gathering/setup.session.json"));
        if load.is_none() && save.try_exists().map_err(|error| error.to_string())? {
            load = Some(save.clone());
        }
        Ok(Some(Self::Live { load, save }))
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Observe {},
    Choose { revision: u64, id: String },
    Save {},
    Reopen {},
    Quit {},
}

fn emit(output: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())
}

fn response(game: &Session, status: &str, error: Option<String>) -> Value {
    let observation = game.observe();
    let mut response = json!({
        "ok": error.is_none(), "mode": "live", "status": status,
        "observation": observation,
    });

    if let Some(error) = error {
        response["error"] = json!(error);
    }
    response
}

pub fn run() -> Result<(), String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let Some(options) = Options::parse()? else {
        return emit(
            &mut output,
            &json!({"ok": true, "status": "help", "usage": USAGE}),
        );
    };
    let mut game = match &options {
        Options::Inspect(path)
        | Options::Live {
            load: Some(path), ..
        } => load_session(path).map_err(|error| error.to_string())?,
        Options::Live { load: None, .. } => Session::new().map_err(|e| e.to_string())?,
    };
    if game.is_verification_fixture() {
        return Err(
            "This record is a privileged verification fixture, not an ordinary Gathering setup."
                .into(),
        );
    }
    let Options::Live { save, .. } = options else {
        return emit(
            &mut output,
            &json!({"ok": true, "mode": "inspection", "status": "inspected", "observation": game.observe()}),
        );
    };
    save_session(&save, &game).map_err(|error| error.to_string())?;

    emit(&mut output, &response(&game, "ready", None))?;
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        let request = match serde_json::from_str::<Request>(&line) {
            Ok(request) => request,
            Err(error) => {
                emit(
                    &mut output,
                    &response(&game, "refused", Some(error.to_string())),
                )?;
                continue;
            }
        };
        match request {
            Request::Observe {} => emit(
                &mut output,
                &response(&game, "observed", None),
            )?,
            Request::Choose { revision, id } => {

                match commit_choice(&save, &mut game, revision, &id) {
                    Ok(()) => emit(&mut output, &response(&game, "chosen", None))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Save {} => {

                match save_session(&save, &game) {
                    Ok(()) => emit(&mut output, &response(&game, "saved", None))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Reopen {} => {

                match load_session(&save) {
                    Ok(saved) if !saved.is_verification_fixture() => {
                        game = saved;
                        emit(&mut output, &response(&game, "reopened", None))?;
                    }
                    Ok(_) => emit(&mut output, &response(&game, "refused", Some("A privileged verification fixture cannot replace the live player setup.".into())))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Quit {} => {
                emit(&mut output, &response(&game, "bye", None))?;
                break;
            }
        }
    }
    Ok(())
}
