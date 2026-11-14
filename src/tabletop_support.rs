//! Artwork settings and text shared by the scenario presentation.
pub(crate) mod asset_settings;
pub(crate) mod symbols;
pub(crate) fn readable_text(value: String) -> String {
    value
        .replace(['−', '–'], "-")
        .replace('§', "section ")
        .replace('·', " / ")
        .replace('×', "x")
        .replace('—', " - ")
        .replace(['→', '▸'], ">")
        .replace('▾', "v")
        .replace('←', "<")
        .replace('…', "...")
}

/// `value` in the characters the interface's text face draws, which are printable
/// ASCII: the spelling of [`readable_text`], typographic quotes, apostrophes,
/// dashes and spaces as their plain forms, invisible joiners dropped, and any other
/// character the face lacks as `?`. For text the table does not author, such as
/// what players say at the table (#209, S2).
pub(crate) fn drawable_text(value: &str) -> String {
    readable_text(value.to_owned())
        .chars()
        .filter_map(|character| match character {
            ' '..='~' | '\n' => Some(character),
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => Some('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' | '«' | '»' => {
                Some('"')
            }
            '\u{2010}'..='\u{2015}' => Some('-'),
            '\u{2022}' => Some('*'),
            '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' => None,
            other if other.is_whitespace() => Some(' '),
            _ => Some('?'),
        })
        .collect()
}
