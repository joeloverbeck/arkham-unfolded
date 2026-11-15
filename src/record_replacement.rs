//! Shared filesystem lifecycle; save schemas and admission remain in `saved_state`.

pub(crate) use unfolded_record_replacement::{Replacement, previous_path, read, replace};
