# Legacy empty commitment

`jacqueline-empty-v1.session.json` was produced by the unmodified Jacqueline driver at `cfd57848aba9cd8a74f527b8cd15ab334cd26ac0`, using a disposable `--patrice-away` session and an explicit save. It contains the old empty commitment decision. No personal save or configuration was used.

SHA-256: `81f14273a38cb9b1dda916fe70c7198aaeb9361e09d4888cda8ad3cbbd3ffbd9`.

The continuity and driver tests use these fixed bytes to verify compatible live continuation, unchanged historical replay and preservation on load, manual save and failed persistence. Do not regenerate the fixture from the new implementation.
