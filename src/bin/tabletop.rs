use arkham_unfolded_experiment::{gathering_tabletop, scenario_selection};

fn main() {
    let gathering = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "gathering"]);
    // An agent-seat launch must never fall through to scenario selection, which
    // opens a window: the Gathering launch refuses its incomplete options as text.
    let result = if gathering || gathering_tabletop::agent_seat_requested() {
        gathering_tabletop::run()
    } else {
        scenario_selection::run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
