use arkham_unfolded_experiment::{gathering_tabletop, scenario_selection};

fn main() {
    let gathering = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "gathering"]);
    let mut arguments = std::env::args().skip(1);
    let result = if arguments.next().as_deref() == Some("--decorative-report") {
        arkham_unfolded_experiment::decorative_report(arguments)
    } else if gathering || gathering_tabletop::agent_seat_requested() {
        // An agent-seat launch must never fall through to scenario selection, which
        // opens a window: the Gathering launch refuses its incomplete options as text.
        gathering_tabletop::run()
    } else {
        scenario_selection::run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
