use arkham_unfolded_experiment::party::Session;
use arkham_unfolded_experiment::party::continuity::{commit_choice, load_session, save_session};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const USAGE: &str = "arkham-driver --experiment party [--load PATH] [--save PATH]\nJSON lines: observe; choose with revision and id; save; quit. Random draws are session-owned and not player-selectable.";

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
                    if args.next().as_deref() != Some("party") {
                        return Err("--experiment must be party".into());
                    }
                }
                "--load" => load = Some(PathBuf::from(args.next().ok_or("--load needs a path")?)),
                "--save" => save = Some(PathBuf::from(args.next().ok_or("--save needs a path")?)),
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        let save = save
            .or_else(|| load.clone())
            .unwrap_or_else(|| PathBuf::from("local/experiment/party.session.json"));
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
    let mut game = match &options.load {
        Some(path) => load_session(path).map_err(|error| error.to_string())?,
        None => Session::new().map_err(|e| e.to_string())?,
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
