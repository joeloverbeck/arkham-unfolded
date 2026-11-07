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
