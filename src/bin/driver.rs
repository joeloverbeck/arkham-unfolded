#[path = "driver/gathering.rs"]
mod gathering;
fn main() {
    if let Err(error) = gathering::run() {
        println!(
            "{}",
            serde_json::json!({"ok": false, "status": "error", "error": error})
        );
        std::process::exit(1);
    }
}
