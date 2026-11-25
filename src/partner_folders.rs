//! Where the agent partners' folders and their client program lie, and the instructions
//! the folders hold (Cuba Libre #202). `scripts/tools partner-folder` writes them and the
//! game looks at them, each with this one file, so that neither differs.

use std::path::{Path, PathBuf};
use unfolded_agent_players::folder::CODEX_WAIT;

/// The client program's name.
pub const CLIENT: &str = "arkham-partner";

/// The partner folders and the client program, in Arkham's data directory.
pub struct Places {
    /// Claude Code's folder.
    pub claude_code: PathBuf,
    /// Codex's folder.
    pub codex: PathBuf,
    /// The client program.
    pub client: PathBuf,
}

/// The directory of the two folders in `data`.
pub fn folders(data: &Path) -> PathBuf {
    data.join("partner")
}

impl Places {
    pub fn of(data: &Path) -> Self {
        Self {
            claude_code: folders(data).join("claude-code"),
            codex: folders(data).join("codex"),
            client: data.join("client").join(CLIENT),
        }
    }

    /// The partner instructions with the program's path in place of its name, and the
    /// shared library's words for how Codex waits.
    pub fn instructions(&self) -> String {
        include_str!("../docs/agents/partner-instructions.md")
            .replace(CLIENT, &self.client.display().to_string())
            .replace("{CODEX_WAIT}", CODEX_WAIT)
    }
}

/// Arkham's data directory: `arkham-unfolded` in `$XDG_DATA_HOME`, or in `~/.local/share`.
pub fn data() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .filter(|path| path.is_absolute())
        .map(|data| data.join("arkham-unfolded"))
}
