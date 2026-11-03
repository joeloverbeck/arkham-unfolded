//! Parent choices retain each nested replacement's surviving physical members.

use super::*;

fn positions(tokens: &TokenResolution) -> Vec<(usize, Vec<usize>)> {
    tokens
        .root
        .at(&tokens.active)
        .children
        .iter()
        .enumerate()
        .map(|(position, child)| (position, retained(child, tokens)))
        .filter(|(_, members)| !members.is_empty())
        .collect()
}

pub(super) fn choices(game: &Game, tokens: &TokenResolution) -> Option<Vec<Choice>> {
    let node = tokens.root.at(&tokens.active);
    let positions = positions(tokens);
    if !positions.iter().any(|(_, members)| members.len() > 1)
        || (tokens.stage == TokenStage::Selection && node.eyes_with_groups())
        || !matches!(
            tokens.stage,
            TokenStage::Cancellation | TokenStage::Selection
        )
    {
        return None;
    }
    let cancellation = tokens.stage == TokenStage::Cancellation;
    let eligible: Vec<_> = positions
        .iter()
        .filter(|(_, members)| {
            !cancellation
                || members.iter().any(|&index| {
                    !protected(game, Investigator::Jacqueline, tokens.revealed[index])
                })
        })
        .collect();
    let selected = |members: &[usize]| members.iter().all(|index| tokens.selected.contains(index));
    let auto = |members: &[usize]| {
        members
            .iter()
            .any(|&index| tokens.revealed[index] == TokenFace::AutoFail)
    };
    let preferred = |members: &[usize]| {
        members.iter().any(|&index| {
            symbol(tokens.revealed[index]) && tokens.revealed[index] != TokenFace::ElderSign
        })
    };
    let selected_count = eligible
        .iter()
        .filter(|(_, members)| selected(members))
        .count();
    let auto_selected = eligible
        .iter()
        .any(|(_, members)| selected(members) && auto(members));
    let prefer_symbols =
        node.policy == Policy::Prophecy && eligible.iter().any(|(_, members)| preferred(members));
    let required = if cancellation {
        eligible
            .iter()
            .filter(|(_, members)| !auto(members))
            .count()
            .min(2)
    } else if node.policy == Policy::Olive {
        eligible.len().min(2)
    } else {
        1
    };
    let mut result = vec![];
    for (position, members) in eligible {
        let is_selected = selected(members);
        let available = if cancellation {
            !auto_selected && selected_count < 2 && (!auto(members) || selected_count == 0)
        } else {
            selected_count < required && (!prefer_symbols || preferred(members))
        };
        if is_selected || available {
            let faces = members
                .iter()
                .map(|&index| tokens.revealed[index].label())
                .collect::<Vec<_>>()
                .join(", ");
            let operation = if cancellation {
                "cancel-position"
            } else {
                "position"
            };
            let protected_faces = members
                .iter()
                .copied()
                .filter(|&index| {
                    cancellation
                        && protected(game, Investigator::Jacqueline, tokens.revealed[index])
                })
                .map(|index| tokens.revealed[index].label())
                .collect::<Vec<_>>()
                .join(", ");
            let exception = if protected_faces.is_empty() {
                String::new()
            } else {
                format!("; Dark Future keeps {protected_faces}")
            };
            result.push(choice(
                format!("tokens:{operation}:{position}"),
                format!(
                    "{} position {} [{}]{}",
                    if is_selected {
                        "Unselect"
                    } else if cancellation && !protected_faces.is_empty() {
                        "Cancel eligible tokens in"
                    } else if cancellation {
                        "Cancel"
                    } else {
                        "Resolve"
                    },
                    position + 1,
                    faces,
                    exception
                ),
            ));
        }
    }
    if (cancellation && auto_selected) || (selected_count != 0 && selected_count == required) {
        result.push(choice("tokens:confirm", "Confirm selected token positions"));
    }
    Some(result)
}

pub(super) fn toggle(tokens: &mut TokenResolution, id: &str) -> bool {
    let position = id.strip_prefix("tokens:cancel-position:").or_else(|| {
        (!tokens.root.at(&tokens.active).eyes_with_groups())
            .then(|| id.strip_prefix("tokens:position:"))
            .flatten()
    });
    let Some(position) = position else {
        return false;
    };
    let position: usize = position.parse().expect("Offered parent position");
    let members = retained(&tokens.root.at(&tokens.active).children[position], tokens);
    if members.iter().all(|index| tokens.selected.contains(index)) {
        tokens.selected.retain(|index| !members.contains(index));
    } else {
        tokens.selected.extend(members);
        tokens.selected.sort_unstable();
    }
    true
}
