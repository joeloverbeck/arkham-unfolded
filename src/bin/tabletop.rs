fn main() {
    let gathering = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "gathering"]);
    let result = if gathering {
        arkham_unfolded_experiment::gathering_tabletop::run()
    } else {
        arkham_unfolded_experiment::scenario_selection::run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
