//! Server generation counter persisted under the sync root.

use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use log::warn;
use serde::{Deserialize, Serialize};
use sinkd_core::outcome::Outcome;

pub(crate) const GENERATION_HISTORY_TTL_SECS: i64 = 7 * 24 * 3600;
pub(crate) const GENERATION_HISTORY_MAX: usize = 4096;

pub(crate) enum PostApply {
    Applied {
        writer_client_id: String,
        head_generation: u64,
    },
    StaleAtApply {
        head_generation: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct HistoryEntry {
    pub(crate) generation: u64,
    pub(crate) saved_at_unix: i64,
}

#[derive(Debug, Default)]
pub(crate) struct GenerationState {
    pub(crate) current_generation: u64,
    pub(crate) history: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedGeneration {
    current_generation: u64,
    #[serde(default)]
    history: Vec<HistoryEntry>,
}

pub(crate) fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

impl GenerationState {
    pub(crate) fn prune_history(&mut self, now_unix: i64) {
        self.history
            .retain(|e| now_unix - e.saved_at_unix <= GENERATION_HISTORY_TTL_SECS);
        while self.history.len() > GENERATION_HISTORY_MAX {
            self.history.remove(0);
        }
    }

    /// Returns the new head generation.
    pub(crate) fn bump(&mut self, now_unix: i64) -> u64 {
        self.current_generation = self.current_generation.saturating_add(1);
        let g = self.current_generation;
        self.history.push(HistoryEntry {
            generation: g,
            saved_at_unix: now_unix,
        });
        self.prune_history(now_unix);
        g
    }
}

#[must_use]
pub(crate) fn load_generation_state(path: &Path) -> GenerationState {
    let Ok(content) = fs::read_to_string(path) else {
        return GenerationState::default();
    };
    let Ok(p) = toml::from_str::<PersistedGeneration>(&content) else {
        warn!(
            "server: unable to parse generation state '{}'",
            path.display()
        );
        return GenerationState::default();
    };
    let mut st = GenerationState {
        current_generation: p.current_generation,
        history: p.history,
    };
    st.prune_history(now_unix_secs());
    st
}

pub(crate) fn persist_generation_state(path: &Path, state: &GenerationState) -> Outcome<()> {
    let p = PersistedGeneration {
        current_generation: state.current_generation,
        history: state.history.clone(),
    };
    let serialized = toml::to_string(&p).map_err(|e| format!("serialize generation state: {e}"))?;
    fs::write(path, serialized)?;
    Ok(())
}
