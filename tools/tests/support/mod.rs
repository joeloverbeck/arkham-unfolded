use std::{path::Path, process::Command, sync::OnceLock};

pub fn fixture() -> &'static Path {
    static BUILD: OnceLock<tempfile::TempDir> = OnceLock::new();
    BUILD
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/child.rs");
            let output = Command::new("rustc")
                .args(["--edition=2024"])
                .arg(source)
                .arg("-o")
                .arg(directory.path().join("child"))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            directory
        })
        .path()
}
