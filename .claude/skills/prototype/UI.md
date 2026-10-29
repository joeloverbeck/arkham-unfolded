# UI prototype

Compare distinct answers to one visual or interaction question. Use the existing Rust presentation path for executable behavior. Static HTML/CSS/SVG is suitable when visual comparison alone answers the question.

## Process

1. Name the decision and the representative content. Use realistic density, long labels, errors, and empty states where they affect that decision.
2. Produce three structurally different variants, at most five. Change information hierarchy, grouping, or interaction structure; color changes alone do not establish alternatives.
3. For interactive variants, use a small Rust selector, launch argument, or separate prototype build. Give one Cargo command and make the selected variant visible. Keep personal saves and settings isolated.
4. For static variants, use ordinary anchor links and inline CSS/SVG in one self-contained HTML file. Do not author JavaScript, embed an executable non-Rust example, or claim static visuals prove interaction behavior.
5. Show the relevant state and label the question each variant answers. Keep controls and layout close to the intended application context.
6. Coordinate a bounded owner comparison and record observations, limitations, and the selected direction on the implementation issue.
7. Preserve the prototype and accepted decision according to the parent skill's capture rule. Implement only the authorized result.

Do not build a new component library, routing system, or prototype framework. Add only what the comparison requires.
