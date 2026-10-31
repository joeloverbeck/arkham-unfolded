# Encounter-discard compatibility witnesses

These fixed Tony sessions were produced before the encounter-discard implementation,
using the unmodified rules and continuity modules at
`402fc483d8d5653e58aaae53a3430b5a435ae347`. A temporary Rust witness used the public
`Session::with_draws([Zero, Zero])` fixture and the choices
`fight`, `commit:none`, `reveal`, `resolve`, `fight`, `commit:none`, `reveal`.
The first save precedes the second `resolve`; the second follows it.
No personal state was used. These fixtures establish compatibility, not the rules oracle.

| File | SHA-256 |
| --- | --- |
| `before-defeat.session.json` | `c42f9b384e1faf569695dade2d9374608361df1300a699689aee447f34c37678` |
| `after-defeat.session.json` | `3575bd616bfc2668c01aa0ae8f069e7786b0e2b035e00e9c4ae37e6a7689568b` |

Keep these original bytes. Do not regenerate them from the implementation under test.
The existing Jacqueline `result-explanation` witnesses supply her completed revelations.
