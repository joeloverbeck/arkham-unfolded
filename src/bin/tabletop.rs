fn main() {
    let party = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "party"]);
    let tony = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "tony"]);
    let result = if party {
        arkham_unfolded_experiment::party_tabletop::run()
    } else if tony {
        arkham_unfolded_experiment::tony_tabletop::run()
    } else {
        arkham_unfolded_experiment::tabletop::run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
