use arkham_unfolded_experiment::tony::Session;
use arkham_unfolded_experiment::tony::continuity::{
    Replay, commit_choice, load_session, save_session,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "arkham-driver --experiment tony [--load PATH] [--save PATH]\nJSON lines: observe; choose with revision and id; save; replay; next; back; live; export with path; quit. Random draws are session-owned and not player-selectable.";

struct Options {
    load: Option<PathBuf>,
    save: PathBuf,
}
impl Options {
    fn parse() -> Result<Option<Self>, String> {
        let mut args = std::env::args().skip(1);
        let mut load = None;
        let mut save = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => return Ok(None),
                "--experiment" => {
                    if args.next().as_deref() != Some("tony") {
                        return Err("--experiment must be tony".into());
                    }
                }
                "--load" => load = Some(PathBuf::from(args.next().ok_or("--load needs a path")?)),
                "--save" => save = Some(PathBuf::from(args.next().ok_or("--save needs a path")?)),
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        let save = save
            .or_else(|| load.clone())
            .unwrap_or_else(|| PathBuf::from("local/experiment/tony.session.json"));
        if load.is_none() && save.exists() {
            return Err("The save path already exists. Use --load to resume it or choose a new --save path.".into());
        }
        Ok(Some(Self { load, save }))
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Observe {},
    Choose { revision: u64, id: String },
    Save {},
    Replay {},
    Next {},
    Back {},
    Live {},
    Export { path: PathBuf },
    Quit {},
}

fn emit(output: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())
}

fn response(game: &Session, replay: Option<&Replay>, status: &str, error: Option<String>) -> Value {
    let observation = replay.map_or_else(|| game.observe(), |replay| replay.current().clone());
    let warnings: Vec<_> = observation
        .choices
        .iter()
        .filter_map(|choice| {
            observation
                .opportunity_attack(&choice.id)
                .map(|attack| json!({"choice_id": choice.id, "attack": attack}))
        })
        .collect();
    let mut response = json!({
        "ok": error.is_none(), "mode": if replay.is_some() { "replay" } else { "live" }, "status": status,
        "observation": observation, "warnings": warnings,
        "encounter_discard": observation.encounter_discard(),
    });
    if let Some(replay) = replay {
        response["replay"] =
            json!({"index": replay.index(), "length": replay.len(), "harm": replay.harm()});
    }
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
    let mut game = match &options.load {
        Some(path) => load_session(path).map_err(|error| error.to_string())?,
        None => Session::new().map_err(|e| e.to_string())?,
    };
    let mut replay = None;
    emit(&mut output, &response(&game, None, "ready", None))?;
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        let request = match serde_json::from_str::<Request>(&line) {
            Ok(request) => request,
            Err(error) => {
                emit(
                    &mut output,
                    &response(&game, replay.as_ref(), "refused", Some(error.to_string())),
                )?;
                continue;
            }
        };
        match request {
            Request::Observe {} => emit(
                &mut output,
                &response(&game, replay.as_ref(), "observed", None),
            )?,
            Request::Choose { revision, id } => {
                if replay.is_some() {
                    emit(&mut output, &response(&game, replay.as_ref(), "refused", Some("Replay cannot change the match. Use live to return to the current decision.".into())))?;
                    continue;
                }
                match commit_choice(&options.save, &mut game, revision, &id) {
                    Ok(()) => emit(&mut output, &response(&game, None, "chosen", None))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, None, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Save {} => {
                if replay.is_some() {
                    emit(&mut output, &response(&game, replay.as_ref(), "refused", Some("Replay cannot save a past moment. Use live to save the current decision.".into())))?;
                    continue;
                }
                match save_session(&options.save, &game) {
                    Ok(()) => emit(&mut output, &response(&game, None, "saved", None))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, None, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Replay {} => match Replay::new(game.recording()) {
                Ok(record) => {
                    replay = Some(record);
                    emit(
                        &mut output,
                        &response(&game, replay.as_ref(), "replay", None),
                    )?;
                }
                Err(error) => emit(
                    &mut output,
                    &response(&game, replay.as_ref(), "refused", Some(error.to_string())),
                )?,
            },
            Request::Next {} | Request::Back {} => {
                if let Some(cursor) = &mut replay {
                    let moved = if matches!(request, Request::Next {}) {
                        cursor.next()
                    } else {
                        cursor.back()
                    };
                    emit(
                        &mut output,
                        &response(
                            &game,
                            replay.as_ref(),
                            if moved { "replay" } else { "boundary" },
                            None,
                        ),
                    )?;
                } else {
                    emit(
                        &mut output,
                        &response(
                            &game,
                            None,
                            "refused",
                            Some("Start replay before navigating its moments.".into()),
                        ),
                    )?;
                }
            }
            Request::Live {} => {
                replay = None;
                emit(&mut output, &response(&game, None, "live", None))?;
            }
            Request::Export { path } => {
                if replay.is_some() {
                    emit(
                        &mut output,
                        &response(
                            &game,
                            replay.as_ref(),
                            "refused",
                            Some("Return to live before exporting the complete recording.".into()),
                        ),
                    )?;
                    continue;
                }
                let result = arkham_unfolded_experiment::tony::continuity::save_recording(
                    path,
                    &game.recording(),
                );
                emit(
                    &mut output,
                    &response(
                        &game,
                        None,
                        if result.is_ok() {
                            "exported"
                        } else {
                            "refused"
                        },
                        result.err().map(|e| e.to_string()),
                    ),
                )?;
            }
            Request::Quit {} => {
                emit(&mut output, &response(&game, replay.as_ref(), "bye", None))?;
                break;
            }
        }
    }
    Ok(())
}
