# HTML Report Format

Render the review as one self-contained static HTML file in the OS temp directory. Use inline CSS and SVG. No authored JavaScript, CDN scripts, runtime diagram renderer, or non-Rust generation program is needed.

## Scaffold

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Architecture review for {{repo name}}</title>
  <style>
    body { margin: 0; background: #fafaf9; color: #0f172a; font: 16px/1.5 system-ui, sans-serif; }
    main { max-width: 68rem; margin: auto; padding: 3rem 1.5rem; }
    article { margin: 2.5rem 0; padding: 1.5rem; border: 1px solid #cbd5e1; background: white; }
    .comparison { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: 1.5rem; }
    .badge { display: inline-block; padding: .15rem .6rem; border-radius: .3rem; background: #e2e8f0; }
    .strong { background: #d1fae5; } .explore, .adr { background: #fef3c7; }
    .files { font: .875rem/1.5 monospace; overflow-wrap: anywhere; }
    svg { width: 100%; height: auto; } svg text { font: 14px system-ui, sans-serif; }
    .module { fill: white; stroke: #475569; stroke-width: 2; }
    .deep { fill: #0f172a; stroke: #0f172a; stroke-width: 4; }
    .seam { stroke-dasharray: 4 4; } .leak { stroke: #dc2626; }
  </style>
</head>
<body>
  <main>
    <header><h1>{{repo name}}</h1><p>{{date}} · Solid box: module · Dashed line: seam · Red arrow: leakage · Thick dark box: deep module</p></header>
    <section id="candidates"><!-- Candidate articles go here. --></section>
    <section id="top-recommendation"><!-- Name, reason, and anchor link. --></section>
  </main>
</body>
</html>
```

## Candidate card

Each candidate is an `<article>` with a stable anchor ID:

- **Title:** name the proposed deepening.
- **Badges:** recommendation strength (`Strong`, `Worth exploring`, or `Speculative`) and dependency category (`in-process`, `local-substitutable`, `ports & adapters`, or `mock`).
- **Files:** a monospaced list using the `files` class.
- **Before / After:** two labeled figures in a `comparison` container. These carry the explanation.
- **Problem and solution:** one sentence each.
- **Wins:** short bullets, at most six words each.
- **ADR callout:** one line when an existing decision constrains the candidate.

Use plain prose and the vocabulary from `/codebase-design`. If a diagram needs a paragraph to explain it, redraw it.

## Diagram patterns

Use the pattern that explains the candidate. SVG text remains text, with a descriptive accessible title and a responsive `viewBox`. Keep identifiers unique when repeating diagrams.

### Dependencies and call flow

Draw modules and arrows directly. For example:

```html
<svg viewBox="0 0 500 180" role="img" aria-labelledby="intake-after-title">
  <title id="intake-after-title">After: the caller enters one deep intake module</title>
  <defs><marker id="intake-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 L8 4 L0 8 Z" fill="#475569"/></marker></defs>
  <rect class="module" x="10" y="55" width="125" height="60" rx="4"/>
  <text x="72" y="90" text-anchor="middle">Caller</text>
  <path d="M135 85 H230" fill="none" stroke="#475569" stroke-width="2" marker-end="url(#intake-arrow)"/>
  <rect class="deep" x="240" y="25" width="245" height="120" rx="4"/>
  <text x="362" y="80" text-anchor="middle" fill="white">Intake module</text>
  <text x="362" y="105" text-anchor="middle" fill="#cbd5e1">Validation and persistence</text>
</svg>
```

Use red arrows for leakage and dashed lines for seams. A sequence can use labeled horizontal arrows between vertical lifelines; no runtime layout engine is required.

### Cross-section

Stack horizontal bands to show the modules a call passes through. Contrast several shallow modules with one module owning the consolidated responsibility.

### Mass diagram

Draw two labeled rectangles per module: interface and implementation. Show their relative size before and after deepening. The sizes are explanatory, not measured complexity unless supported by evidence.

### Call-graph collapse

Show a tree of calls as nested boxes before; afterward, enclose the internal calls inside one thick module, with the public entry clearly marked.

## Style and recommendation

Use generous whitespace, one accent, red for leakage, and amber for unresolved constraints. Keep before/after figures similar in height and readable on narrow screens. Use ordinary anchor links for navigation.

End with one larger recommendation card: candidate name, one sentence explaining why, and a link to its candidate card. Use the precise terms module, interface, implementation, depth, deep, shallow, seam, adapter, leverage, and locality where they apply. State concrete gains rather than generic claims of maintainability.
