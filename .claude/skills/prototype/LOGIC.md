# Logic prototype

Answer one explicit question about a state model with a small Rust example the owner can drive. The implementation-language contract applies even when the artifact is temporary.

## Process

1. State the question, the relevant states and actions, and the observation that would support or reject the proposed model.
2. Keep transitions in a small Rust module. Use the existing presentation path when clicks and visible feedback matter; a terminal interaction is sufficient only when it answers the agreed question.
3. Give one Cargo command. Keep state in memory, with a visible reset action. If persistence is itself under investigation, use an explicitly disposable location.
4. Show the current domain state and the effects of each action. Use domain terminology, not internal implementation names.
5. Provide free actions and a few guided cases covering the difficult transitions, including invalid actions and interruption where relevant. Explain what each case tests and what remains unproven.
6. Coordinate a bounded owner inspection. Record the observed result and the owner's decision on the implementation issue. A successful launch does not establish that the model feels right.
7. Preserve the question, evidence, and prototype according to the parent skill's capture rule. Integrate only the accepted decision within its authorized scope.

Static HTML, CSS, or SVG may explain states, but does not prove executable transitions. Do not generate a JavaScript state machine or a separate non-Rust simulation.

Keep the prototype small. Add only the controls and feedback needed to answer the question; do not grow a reusable authoring framework.
