//! Public descriptions derived from the live reveal tree. No second rules state.
use super::{Policy, RevealNode, TokenPurpose, TokenResolution, TokenStage, retained};

/// Physical provenance. Membership never implies current choice eligibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevealGroup {
    pub source: String,
    pub members: Vec<usize>,
}

impl TokenResolution {
    /// One plain draw: Jacqueline's whole-reveal and first-draw scopes have
    /// identical consequences, including under later Olive McBride, Dark
    /// Prophecy or Katarina Sojka declarations, so only the whole reveal is
    /// offered. Eyes of the Dreamer is excluded even without charges: under the
    /// first-draw scope her group is one Eyes candidate.
    pub(super) fn single_position(&self) -> bool {
        self.root.policy == Policy::All
            && !self.root.jacqueline
            && self.root.children == [RevealNode::draw()]
    }

    /// Jacqueline's offered scope, described by what it does. Arbiter of Fates
    /// pays instead of her round use.
    pub(super) fn jacqueline_label(&self, path: &[usize], arbiter: bool) -> String {
        let add = "add 2 tokens, then cancel";
        let eyes = self.root.policy == Policy::Eyes;
        let scope = match path.split_last() {
            None if self.single_position() => " — reveal 3 tokens, then cancel".to_owned(),
            None => {
                let around = match self.root.children.as_slice() {
                    [only] => group(only.policy),
                    _ => None,
                };
                if let Some((name, keep)) = around {
                    format!(
                        " around {name} — choose {name}'s {keep} first; then cancel whole results"
                    )
                } else {
                    let affect = match self.root.children.len() {
                        1 => "affect the whole reveal".to_owned(),
                        2 => "affect both draws".to_owned(),
                        draws => format!("affect all {draws} draws"),
                    };
                    if eyes {
                        format!(
                            " — {affect}: add 2 tokens as separate Eyes candidates, then cancel"
                        )
                    } else {
                        format!(" — {affect}: {add}")
                    }
                }
            }
            Some((&index, parent)) => {
                let container = self.root.at(parent);
                let node = &container.children[index];
                let draw = index + 1;
                match (group(node.policy), group(container.policy)) {
                    (Some((name, keep)), _) => format!(
                        " on {name}'s whole draw — add 2 tokens to {name}'s {}; cancel first, then choose {keep}",
                        node.children.len()
                    ),
                    (None, Some((name, _))) => {
                        format!(" on {name}'s draw {draw} — cancel within this draw first")
                    }
                    _ if eyes => format!(
                        " — affect only draw {draw}: {add}; her group is one Eyes candidate"
                    ),
                    _ => format!(" — affect only draw {draw}: {add}"),
                }
            }
        };
        if arbiter {
            format!("Use Jacqueline + Arbiter{scope}; exhaust Arbiter, preserve round use")
        } else {
            format!("Use Jacqueline{scope}")
        }
    }

    pub fn reveal_groups(&self) -> Vec<RevealGroup> {
        fn collect(
            node: &RevealNode,
            source: String,
            location: String,
            out: &mut Vec<RevealGroup>,
        ) {
            let source =
                policy_label(node.policy).map_or(source, |name| format!("{name} · {location}"));
            // In an ordinary Jacqueline reveal every draw has the same scope.
            // Nested replacements retain their distinct provenance below.
            if node.jacqueline
                && node.policy == Policy::All
                && node.children.len() == 3
                && node.children.iter().any(|child| !child.drawn.is_empty())
                && node
                    .children
                    .iter()
                    .all(|child| child.policy == Policy::Draw && child.children.is_empty())
            {
                out.push(RevealGroup {
                    source: format!("Jacqueline · {location}"),
                    members: node
                        .children
                        .iter()
                        .flat_map(|child| child.drawn.iter().copied())
                        .collect(),
                });
                return;
            }
            if node.children.is_empty() {
                if !node.drawn.is_empty() {
                    if let Some(group) = out.iter_mut().find(|group| group.source == source) {
                        group.members.extend(&node.drawn);
                    } else {
                        out.push(RevealGroup {
                            source,
                            members: node.drawn.clone(),
                        });
                    }
                }
                return;
            }
            for (index, child) in node.children.iter().enumerate() {
                let extra = node.jacqueline && index >= node.children.len() - 2;
                let child_source = if extra {
                    format!("Jacqueline · added to {location}")
                } else {
                    source.clone()
                };
                let target = if extra {
                    format!(
                        "{location} · Jacqueline extra {}",
                        index + 3 - node.children.len()
                    )
                } else {
                    format!("{location} · draw {}", index + 1)
                };
                collect(child, child_source, target, out);
            }
        }
        let mut groups = Vec::new();
        for (index, root) in self.previous_roots.iter().chain([&self.root]).enumerate() {
            let source = if index == 0 {
                self.purpose_label().to_owned()
            } else {
                format!("Curse · additional reveal {index}")
            };
            collect(root, source.clone(), source, &mut groups);
        }
        groups
    }

    /// Maps an offered decision to physical members, including nested groups.
    /// Callers still use the live choice list as the authority for eligibility.
    pub fn choice_members(&self, id: &str) -> Vec<usize> {
        if let Some(index) = ["tokens:keep:", "tokens:cancel:", "tokens:effect:"]
            .into_iter()
            .find_map(|prefix| id.strip_prefix(prefix))
            .and_then(|value| value.parse::<usize>().ok())
        {
            return (index < self.revealed.len())
                .then_some(index)
                .into_iter()
                .collect();
        }
        if let Some(position) = ["tokens:position:", "tokens:cancel-position:"]
            .into_iter()
            .find_map(|prefix| id.strip_prefix(prefix))
            .and_then(|value| value.parse::<usize>().ok())
        {
            return self
                .root
                .at(&self.active)
                .children
                .get(position)
                .map_or_else(Vec::new, |child| retained(child, self));
        }
        Vec::new()
    }

    pub fn instruction(&self) -> String {
        let node = self.root.at(&self.active);
        let target = self.target_label(&self.active);
        let grouped = node
            .children
            .iter()
            .any(|child| retained(child, self).len() > 1);
        let unit = if grouped { "reveal group" } else { "token" };
        match self.stage {
            TokenStage::Declaration => format!(
                "{} · Plan the reveal before seeing any tokens. Choose an ability and the reveal it changes, or reveal now.",
                self.purpose_label()
            ),
            TokenStage::EyesCharges => {
                "Eyes of the Dreamer · Choose how many charges to spend before revealing.".into()
            }
            TokenStage::Cancellation => format!(
                "Jacqueline · {target} — choose {unit}s to cancel\nCancel 2 non-auto-fail {unit}s, or 1 auto-fail {unit}. If fewer are eligible, cancel as many as possible. Then confirm."
            ),
            TokenStage::Selection => {
                let count = if node.policy == Policy::Olive { 2 } else { 1 };
                let preference = if node.policy == Policy::Prophecy {
                    " A symbol other than Elder Sign is required when available."
                } else {
                    ""
                };
                format!(
                    "{target}\nChoose {count} {unit}{} to resolve.{preference} Then confirm.",
                    if count == 1 { "" } else { "s" }
                )
            }
            TokenStage::Effects => format!(
                "{} · Choose the next token consequence to resolve.",
                self.purpose_label()
            ),
            TokenStage::Resolving => {
                format!("{} · Resolving the revealed tokens.", self.purpose_label())
            }
        }
    }

    pub fn selection_progress(&self, choices: &[crate::Choice]) -> Option<String> {
        if !matches!(self.stage, TokenStage::Selection | TokenStage::Cancellation) {
            return None;
        }
        let node = self.root.at(&self.active);
        let grouped = choices.iter().any(|choice| {
            choice.id.starts_with("tokens:position:")
                || choice.id.starts_with("tokens:cancel-position:")
        });
        let selected = if grouped {
            node.children
                .iter()
                .map(|child| retained(child, self))
                .filter(|members| {
                    !members.is_empty() && members.iter().all(|index| self.selected.contains(index))
                })
                .count()
        } else {
            self.selected.len()
        };
        let available = choices
            .iter()
            .filter(|choice| !self.choice_members(&choice.id).is_empty())
            .count();
        let auto = self.stage == TokenStage::Cancellation
            && self
                .selected
                .iter()
                .any(|&index| self.revealed[index] == crate::TokenFace::AutoFail);
        let required = if auto {
            1
        } else if self.stage == TokenStage::Cancellation || node.policy == Policy::Olive {
            2
        } else {
            1
        };
        Some(format!(
            "Selected {selected} of {} {}{}",
            required.min(available.max(selected)),
            if grouped { "reveal group" } else { "token" },
            if required.min(available.max(selected)) == 1 {
                ""
            } else {
                "s"
            }
        ))
    }

    pub fn reveal_plan(&self) -> Vec<String> {
        let mut nodes = vec![];
        self.root.declarations(vec![], &mut nodes);
        nodes
            .into_iter()
            .filter_map(|(path, policy, jacqueline)| {
                let source = policy_label(policy);
                (source.is_some() || jacqueline).then(|| {
                    if policy == Policy::Olive {
                        if self.root.at(&path).children.iter().any(|child| child.policy != Policy::Draw) {
                            return format!("Olive active — resolve affected draws first, then choose 2 results · {}", self.target_label(&path));
                        }
                        return format!(
                            "Olive active — reveal {}, resolve 2{}",
                            if jacqueline { 5 } else { 3 },
                            if jacqueline {
                                "; Jacqueline cancels first"
                            } else {
                                ""
                            }
                        );
                    }
                    format!(
                        "{}{}",
                        self.target_label(&path),
                        if jacqueline {
                            " · Jacqueline adds 2, then cancels"
                        } else {
                            ""
                        }
                    )
                })
            })
            .collect()
    }

    fn purpose_label(&self) -> &'static str {
        match self.purpose {
            TokenPurpose::SkillTest => "Skill test",
            TokenPurpose::VoiceRa { .. } => "Voice of Ra",
            TokenPurpose::Premonition { .. } => "Premonition",
            TokenPurpose::DarkFuture { .. } => "Dark Future",
        }
    }

    pub(super) fn target_label(&self, path: &[usize]) -> String {
        let mut label = self.purpose_label().to_owned();
        let mut node = &self.root;
        if path.is_empty() {
            return policy_label(node.policy).map_or_else(
                || format!("{label} · whole reveal"),
                |source| format!("{source} · {label} · whole reveal"),
            );
        }
        for &index in path {
            if let Some(source) = policy_label(node.policy) {
                label = format!("{source} within {label}");
            }
            if node.jacqueline && index >= node.children.len() - 2 {
                label.push_str(&format!(
                    " · Jacqueline extra {}",
                    index + 3 - node.children.len()
                ));
            } else {
                label.push_str(&format!(" · draw {}", index + 1));
            }
            node = &node.children[index];
        }
        if let Some(source) = policy_label(node.policy) {
            label = format!("{source} replacing {label}");
        }
        label
    }
}

fn policy_label(policy: Policy) -> Option<&'static str> {
    match policy {
        Policy::Draw | Policy::All => None,
        Policy::Sealed => Some("Premonition"),
        Policy::Prophecy => Some("Dark Prophecy"),
        Policy::Olive => Some("Olive McBride"),
        Policy::Katarina => Some("Katarina Sojka"),
        Policy::Eyes => Some("Eyes of the Dreamer"),
    }
}

/// Replacement groups whose controller chooses results from their draws.
fn group(policy: Policy) -> Option<(&'static str, u8)> {
    match policy {
        Policy::Olive => Some(("Olive", 2)),
        Policy::Prophecy => Some(("Dark Prophecy", 1)),
        _ => None,
    }
}
