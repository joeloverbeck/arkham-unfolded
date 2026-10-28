fn main() {
    let tony = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["--experiment", "tony"]);
    let result = if tony {
        arkham_unfolded_experiment::tony_tabletop::run()
    } else {
        arkham_unfolded_experiment::tabletop::run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
