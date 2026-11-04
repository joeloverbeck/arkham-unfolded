#[path = "driver/gathering.rs"]
mod gathering;
#[path = "driver/party.rs"]
mod party;
#[path = "driver/tony.rs"]
mod tony;

use arkham_unfolded_experiment::continuity::{commit_choice, load_session, save_session};
use arkham_unfolded_experiment::{Fixture, Session};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "arkham-driver [--case curse|autofail] [--patrice-away] [--used-jacqueline] [--load PATH] [--save PATH]\nJSON lines: observe; choose with revision and id; save; quit. Fixture options cannot accompany --load.";

struct Options {
    fixture: Fixture,
    load: Option<PathBuf>,
    save: PathBuf,
}

impl Options {
    fn parse() -> Result<Option<Self>, String> {
        let mut args = std::env::args().skip(1);
        let mut case = "curse".to_owned();
        let mut away = false;
        let mut used = false;
        let mut fixture_selected = false;
        let mut load = None;
        let mut save = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => return Ok(None),
                "--case" => {
                    case = args.next().ok_or("--case needs curse or autofail")?;
                    fixture_selected = true;
                }
                "--patrice-away" => {
                    away = true;
                    fixture_selected = true;
                }
                "--used-jacqueline" => {
                    used = true;
                    fixture_selected = true;
                }
                "--load" => load = Some(PathBuf::from(args.next().ok_or("--load needs a path")?)),
                "--save" => save = Some(PathBuf::from(args.next().ok_or("--save needs a path")?)),
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if load.is_some() && fixture_selected {
            return Err("--load cannot be combined with fixture options".into());
        }
        let mut fixture = match case.as_str() {
            "curse" => Fixture::curse(),
            "autofail" => Fixture::auto_fail(),
            _ => return Err("--case must be curse or autofail".into()),
        };
        if away {
            fixture = fixture.with_patrice_elsewhere();
        }
        if used {
            fixture = fixture.with_jacqueline_used();
        }
        let save = save
            .or_else(|| load.clone())
            .unwrap_or_else(|| PathBuf::from("local/experiment/session.json"));
        if load.is_none() && save.exists() {
            return Err("The save path already exists. Use --load to resume it or choose a new --save path.".into());
        }
        Ok(Some(Self {
            fixture,
            load,
            save,
        }))
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Observe {},
    Choose { revision: u64, id: String },
    Save {},
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
        "encounter_discard": observation.encounter_discard(),
    });

    if let Some(error) = error {
        response["error"] = json!(error);
    }
    response
}

fn run() -> Result<(), String> {
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
        None => Session::new(options.fixture),
    };

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
            Request::Observe {} => emit(&mut output, &response(&game, "observed", None))?,
            Request::Choose { revision, id } => {
                match commit_choice(&options.save, &mut game, revision, &id) {
                    Ok(()) => emit(&mut output, &response(&game, "chosen", None))?,
                    Err(error) => emit(
                        &mut output,
                        &response(&game, "refused", Some(error.to_string())),
                    )?,
                }
            }
            Request::Save {} => match save_session(&options.save, &game) {
                Ok(()) => emit(&mut output, &response(&game, "saved", None))?,
                Err(error) => emit(
                    &mut output,
                    &response(&game, "refused", Some(error.to_string())),
                )?,
            },
            Request::Quit {} => {
                emit(&mut output, &response(&game, "bye", None))?;
                break;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let gathering_mode = args
        .windows(2)
        .any(|pair| pair == ["--experiment", "gathering"]);
    let party_mode = args
        .windows(2)
        .any(|pair| pair == ["--experiment", "party"]);
    let result = if gathering_mode {
        gathering::run()
    } else if party_mode {
        party::run()
    } else if args.iter().any(|arg| arg == "--experiment") {
        tony::run()
    } else {
        run()
    };
    if let Err(error) = result {
        let _ = emit(
            &mut io::stdout().lock(),
            &json!({"ok": false, "status": "error", "error": error}),
        );
        std::process::exit(1);
    }
}
