# Contextual card controls: radial menus and local action panels

Research completed on 2026-10-29 for the owner's `grill-with-docs` interview related to [#22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) and [ADR 0002](../adr/0002-persistent-table-areas.md). The code inspected is `4254f50d228399379cfeccf6917d5cf5fa7a0cfb`. This is decision support, not an implemented interaction or evidence of accepted native usability. The bounded question is how to expose a selected component's legal actions near that component while preserving readable consequences and deliberate choices.

After this research, Joel selected the recommended labeled radial controls with a local-list fallback; ADR 0002 records that design decision. The evidence and native-use limitations below remain unchanged by selecting the direction.

On 2026-11-01, Joel replaced that hybrid menu shape with universal vertical action rows after reporting overlapping detailed labels in Party. See the [follow-up research](party-interaction-refinements.md) and [accepted refinement](../adr/0002-persistent-table-areas.md#vertical-action-rows-and-commitment-eligibility). This earlier research remains historical evidence.

Joel subsequently selected retaining the existing card inspection modal, opened by Inspect in the contextual menu, because it already works well for him. The visual-context recommendations here govern routine action selection; they do not require replacing deliberate full inspection with a nonmodal panel. Neither inspection nor its dismissal commits a game choice.

## Recommendation

Use a **small, labeled contextual action cluster** next to the selected card. A sparse radial arrangement is a reasonable first visual candidate for its few short commands; use a compact local list when wording or available screen space requires it. Preserve the same explicit selection behavior in either presentation. Avoid a large filled wheel, icon-only choices, deep radial hierarchies, and invisible marking gestures in the first implementation.

The recommendation is an application-specific inference from the evidence below and the actual command inventory. The studies do not establish that radial controls are universally best, or that this proposed cluster is faster than a well-placed local list. The strongest and most direct change is moving component actions into their visual context. Menu shape is a secondary choice.

The existing sidebar is the incumbent. A compact card-adjacent list is the least-complex credible comparison: it removes the long trip to the sidebar while preserving readable labels and familiar clicking. A radial cluster earns its extra layout work only if it remains clear and comfortable in the actual scene. Neither alternative needs a general menu framework or new game semantics.

## What the primary evidence establishes

**Pie versus linear menus.** Callahan, Hopkins, Weiser and Shneiderman's CHI 1988 experiment used 33 undergraduates with little or no mouse experience and fixed eight-item popup menus. Pie menus had faster selection; the error-rate difference was only marginal (`p = .087`), and preferences were divided. Both presentations used press-and-release selection. The authors identify screen coverage, occlusion and untested menu lengths as limitations. This supports testing short directional menus, not a universal speed, accuracy or eight-item-limit claim. [Original paper, pp. 97–100](https://www.cs.umd.edu/~ben/papers/Callahan1988empirical.pdf)

**Learned gestures depend on stable commands.** Kurtenbach and Buxton's CHI 1994 field study followed two users performing six frequent editing commands. Learned marks became faster than displayed-menu selection, but actual selection errors were not measured objectively. Users returned to displayed menus after breaks. The authors explicitly warn that changing item locations prevents users from learning the corresponding marks. These conditions differ from Arkham's changing legal choices and occasional consequential decisions. [Author copy, study and conclusions](https://www.billbuxton.com/MMUserLearn.html)

**A radial menu need not be a filled pie.** Tapia and Kurtenbach's UIST 1995 refinements replace large wedges with horizontal labels in small opaque boxes around an open center. They describe preserving underlying context and accommodating longer labels. Their refinements came from iterative work and feedback from about 15 users, without a formal comparison of those refinements. This is useful design precedent for a sparse labeled cluster, not proof of its superiority here. [Original paper, pp. 190–194](https://www.research.autodesk.com/app/uploads/2023/03/some-design-refinements-and.pdf_recDHrrSOdirknTZs.pdf)

**Edge handling and explicit clicking are separate design choices.** Original implementer Don Hopkins describes both two-click and press-drag-release pie selection. He also documents the problems caused by long labels, changing command directions, and moving a menu away from its invocation point near an edge. His historical cursor-warp solution is not recommended for this application. The transferable issue is preserving an understandable relationship between pointer, menu and source. [Implementer's account, disadvantages and design considerations](https://www.donhopkins.com/home/catalog/piemenus/ddj/piemenus.html)

**Deep gesture hierarchies carry their own limits.** Kurtenbach and Buxton's CHI 1993 experiment simulated experts with memorized compass/clock layouts, using 12 participants. Increasing breadth and depth increased difficulty, particularly for mouse input and off-axis directions. This does not justify layering arbitrary card abilities and target combinations into nested wheels. [Author copy, experiment and conclusions](https://www.billbuxton.com/MMExpert.html)

**Desktop guidance supports small local controls.** GNOME describes popovers as attached overlays and recommends keeping them small and simple, with Escape dismissal. Its menu guidance favors meaningful ordering, concise command labels and limited nesting. These are suitable conventions for an Ubuntu application, not comparative evidence that GTK-style menus outperform radial controls. [GNOME popovers](https://developer.gnome.org/hig/patterns/containers/popovers.html), [GNOME menus](https://developer.gnome.org/hig/patterns/controls/menus.html)

**Cancellation and keyboard behavior deserve explicit treatment.** W3C's pointer-cancellation guidance favors an opportunity to abort before completion; its menu pattern documents keyboard navigation, activation, dismissal and focus restoration. These are useful interaction references, not a claim that a native Bevy application satisfies web accessibility standards. The requirement for a separate action click comes from the owner's accepted selection-never-commits contract, not from WCAG requiring two clicks. [Pointer cancellation](https://www.w3.org/WAI/WCAG22/Understanding/pointer-cancellation), [Menu keyboard pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/)

## Pressure from the current application

The following are static findings in the current source, not observations of a completed contextual-menu implementation:

| Current interaction | Implication for presentation |
| --- | --- |
| Tony's action stage has up to eight rules choices including Undo; most belong to different components. | Do not put the whole turn into a wheel around every card. |
| The Ghoul offers at most two gameplay choices at once: Fight and either Evade or Engage. Injury, Study and deck each have at most one relevant gameplay action. | Individual component menus can be small; adding Inspect still does not require a large wheel. |
| Costs and effects appear in labels, including Injury's two ordinary actions and Promise of Power's added Curse. | A short verb may head a choice, but material cost/consequence text must remain visible before activation. |
| Provoking actions have a visible Ghoul attack warning with damage and horror amounts. | Moving controls must preserve the warning before the action click, rather than burying it in inspection or history. |
| Two copies of On the Hunt can be eligible together; their individual choices have identical labels but distinct instance IDs. | Select identifiable physical copies and preserve the legal combined commitment. Do not execute the first single-card choice while the user is still choosing a subset. |
| Jacqueline's cancellation choices concern pairs of revealed tokens, or the Auto-fail alternative. | Use direct token selection with an explicit completion control, or clearly present the existing complete alternatives; these are not generic card commands. |
| Draw, reveal, and using Jacqueline's ability can reveal hidden information. | Hover, pointer travel or opening a menu cannot execute them. Undo is not a universal recovery mechanism. |

Evidence: [Tony's legal choices](../../src/tony.rs), [Jacqueline's legal choices and ability resolution](../../src/lib.rs), [opportunity-attack previews and current card inspection](../../src/tony_tabletop.rs), and the two-copy commitment case in [Tony decision tests](../../tests/tony_decisions.rs). The principal source locations at the inspected revision are `src/tony.rs:342–403`, `src/lib.rs:314–380` and `:422`, `src/tony_tabletop.rs:466–505` and `:748`, and `tests/tony_decisions.rs:315`.

Current card clicking opens inspection; separate sidebar buttons immediately dispatch a revision-bound choice and autosave its result. Contextual controls therefore need an explicit presentation-only selection stage. The rules' available choices and stale-revision checks remain authoritative. This UI work must neither infer extra legal moves nor discard compound choices for the convenience of a menu shape.

## Proposed interaction

The following recommendations support the selected interaction direction recorded in ADR 0002. Detailed starting choices remain subject to native inspection; the research itself is not proof that they work in this game:

1. **First left click selects.** Highlight the exact card instance and show its name, acting investigator and nearby labeled controls. That click finishes before any action control can activate. Selecting another investigator's card does not silently change the decision actor. Right-drag remains available for camera orbit.
2. **A separate click chooses the action.** Use short labels such as Fight, Evade, Commit and Inspect, with an adjacent readable cost/consequence line when needed. Activate on a completed click that can be canceled by leaving its target. Do not introduce hover execution, drag-release shortcuts or mark-ahead behavior yet.
3. **Keep the cluster sparse.** A small number of stable directions can suit short repeated actions. Four directions is a possible local design choice, not a research-derived maximum. Preserve recurring command positions within a component type rather than continually repacking wedges when legality changes. An unavailable position may stay empty; displaying a disabled action and reason requires actual supporting rules information. Avoid a universal empty eight-slot wheel or arbitrary disabled verbs on every card.
4. **Let text win over geometry.** Long or multiple card-specific abilities belong in a compact list attached to the selected card. Do not shrink fonts, rotate labels, hide costs in icons, or force every ability into a directional slot. Keep Inspect available when no gameplay action applies.
5. **Resolve actual choices explicitly.** A command requiring a target or card subset opens a clearly labeled selection step; it does not immediately commit an incomplete engine choice. Show selected instances and the resulting contribution, then map the completed selection to the exact currently offered choice. Keep decline, continue, turn-ending and other non-component decisions in the persistent prompt area.
6. **Preserve visual context.** Draw controls in readable screen space near the projected card, not tilted on the tabletop. Prefer available space that leaves its name, relevant counters and pending prompt visible. At an edge, translate the whole cluster into the usable viewport and retain a source highlight or short connector. Do not warp the pointer or rotate command directions to make it fit. Use the compact list when the radial layout cannot remain clear.
7. **Make dismissal predictable.** Escape or a click on empty space dismisses without a game action. As a reversible starting rule, camera movement closes the transient menu while retaining card selection; controls should not chase the pointer during a pan. Reopening uses the current observation rather than a stale cached action set. A game revision or mode change invalidates the transient menu and staged selection instead of retargeting an earlier click. Replay remains inspection-only, with no gameplay dispatch until Return to live, preserving the existing branch in `src/tony_tabletop.rs:1106–1114`.
8. **Give the menu keyboard focus.** Offer visible keyboard navigation and activation. While the menu consumes navigation keys, the camera must not also orbit or pan. Restore focus on dismissal. The implementation can choose an appropriate Bevy control pattern; copying ARIA attributes is not an implementation plan.

The outcome sought is that Joel can see a component, understand its available action and consequences, and choose it without searching the distant sidebar or losing the current test. A decorative wheel alone would not establish that outcome.

## Remaining evidence and stopping point

This pass is sufficient to recommend the contextual cluster and to reject claims that a full wheel or hidden gestures are automatically better. It does not settle exact spacing, whether the sparse radial form feels better than the local list, or the burden of camera travel.

Before treating the design as successful, exercise the actual native path with the Ghoul's changing interactions, Injury's cost and attack warning, two distinct On the Hunt copies committed together, and Jacqueline's token alternatives. Include a card near a viewport edge, keyboard dismissal, camera movement and missing artwork. Mechanical checks can establish correct choice dispatch, preservation of legal alternatives and cancellation; only representative owner use can judge readability and effort.

If a concrete visual draft leaves the radial-versus-list choice uncertain, propose one short comparison on those same tasks with identical labels and consequences. Do not build a general menu laboratory or begin repeated owner testing without agreeing its cumulative allowance. Existing #22 acceptance and its already-consumed playtesting allowance are not reset by this design discussion. Stop research here unless a specific unresolved design question requires another source.

Consulted constitutional anchors: `PURP-2`, `PURP-4`, `EVID-2`–`EVID-4`, `AGENT-5` and `OPS-1`. GitHub remains the work-control plane; this note supports the current design decision and creates no independent implementation queue.
