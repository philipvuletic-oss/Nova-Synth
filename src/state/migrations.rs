//! M0 GUI-state migrations and compatibility helpers.

use super::persisted::VersionedState;

pub fn migrate(state: &mut VersionedState) {
    state.migrate();
}
