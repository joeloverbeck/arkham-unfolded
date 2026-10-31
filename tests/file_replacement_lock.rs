//! Keep traced forks in their own test executable, sequentially: fork inherits
//! open file descriptions and can briefly retain another test's flock until exec.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

#[path = "support/install_trace.rs"]
mod install_trace;

#[test]
fn production_drivers_hold_the_same_sidecar_through_both_installations() {
    use std::{fs, os::unix::fs::MetadataExt};
    macro_rules! check {
        ($experiment:ident, $choice:literal) => {{
            use arkham_unfolded_experiment::$experiment::{Session, continuity::*};
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("session.json");
            let lock_path = directory.path().join("session.json.lock");
            let mut game = Session::new().unwrap();
            save_session(&path, &game).unwrap();
            let original = fs::read(&path).unwrap();
            let contender = fs::File::open(&lock_path).unwrap();
            let inode = contender.metadata().unwrap().ino();
            install_trace::driver_choice(
                stringify!($experiment),
                &path,
                $choice,
                |install, finished| {
                    assert_eq!(fs::metadata(&lock_path).unwrap().ino(), inode);
                    assert!(
                        matches!(contender.try_lock(), Err(std::fs::TryLockError::WouldBlock)),
                        "lock must cover install {install}, finished={finished}"
                    );
                    if install == 1 && !finished {
                        assert!(!previous_path(&path).exists());
                    } else {
                        assert_eq!(fs::read(previous_path(&path)).unwrap(), original);
                    }
                    if install == 1 || !finished {
                        assert_eq!(fs::read(&path).unwrap(), original);
                    }
                },
            );
            game.choose(0, $choice).unwrap();
            assert_eq!(load_session(&path).unwrap(), game);
            contender.try_lock().unwrap();
        }};
    }
    check!(tony, "injury");
    check!(party, "select:tony");
}
