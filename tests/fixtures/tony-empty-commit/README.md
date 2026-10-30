# Legacy empty commitment

These fixtures were produced through the unmodified Tony driver at `cfd57848aba9cd8a74f527b8cd15ab334cd26ac0`, using a disposable session. No personal save or configuration was used.

- `pending.session.json`: after Evade, pending the old empty commitment choice. SHA-256 `e5b23ca1947bbece8c797e9269cb2d919d72da13a1a1d1d8f70149152e05ebf7`.
- `pending.recording.json`: the recording exported at that boundary. SHA-256 `e5bbb0594e7f84ce2a895f029b6131de8d5881c172a34e66181d5fb3bbbee480`.
- `declined.session.json`: after choosing `commit:none` from that boundary. SHA-256 `a971b268341ce060adb2f0f18fcaf35559a4699ce199b3f6750b3731ec9e7e89`.

The continuity and driver tests verify compatible live continuation and undo while retaining the original observations, historical prefix and recovery bytes. Do not regenerate these witnesses from the new implementation.
