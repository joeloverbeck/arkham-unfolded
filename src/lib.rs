//! The Gathering rules, persistence and native presentation.

pub mod asset_config;
pub mod assets;
#[cfg(feature = "tabletop")]
mod attempts;
pub mod gathering;
#[cfg(feature = "tabletop")]
pub mod gathering_tabletop;
mod record_replacement;
mod saved_state;
#[cfg(feature = "tabletop")]
pub mod scenario_selection;
#[cfg(feature = "tabletop")]
mod tabletop_camera;
#[cfg(feature = "tabletop")]
mod tabletop_capture;
#[cfg(feature = "tabletop")]
mod tabletop_counters;
#[cfg(feature = "tabletop")]
mod tabletop_feedback;
#[cfg(feature = "tabletop")]
mod tabletop_gathering_map;
#[cfg(feature = "tabletop")]
mod tabletop_harm_text;
#[cfg(feature = "tabletop")]
mod tabletop_harm_tokens;
#[cfg(feature = "tabletop")]
mod tabletop_health;
#[cfg(feature = "tabletop")]
mod tabletop_layout;
#[cfg(feature = "tabletop")]
mod tabletop_pacing;
#[cfg(feature = "tabletop")]
mod tabletop_pieces;
#[cfg(feature = "tabletop")]
mod tabletop_reveal;
#[cfg(feature = "tabletop")]
mod tabletop_skill_text;
#[cfg(all(test, feature = "tabletop"))]
mod tabletop_standee_checks;
#[cfg(feature = "tabletop")]
mod tabletop_standees;
#[cfg(feature = "tabletop")]
mod tabletop_startup;
#[cfg(feature = "tabletop")]
mod tabletop_tokens;
#[cfg(feature = "tabletop")]
mod tabletop_ui;

#[cfg(feature = "tabletop")]
mod tabletop_support;

mod rules_types;
pub use rules_types::{Choice, RuleError, TokenFace, TokenStatus};
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
