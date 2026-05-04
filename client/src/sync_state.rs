//! Client-side persisted sync identity and generation acknowledgement.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use sinkd_core::{ipc, outcome::Outcome};

use crate::parameters::ClientParameters;

pub(crate) struct ClientSyncState {
    pub(crate) client_id: String,
    pub(crate) acked_generation: u64,
    pub(crate) ack_path: PathBuf,
}

pub(crate) fn client_state_dir(params: &ClientParameters) -> PathBuf {
    let shared = &params.shared;
    if shared.debug > 0 {
        if let Some(p) = &params.client_state_dir_override {
            if !p.as_os_str().is_empty() {
                return p.clone();
            }
        }
        if let Ok(p) = std::env::var("SINKD_CLIENT_STATE_DIR") {
            let p = p.trim();
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
        PathBuf::from("/tmp/sinkd/client")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\sinkd\client")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/sinkd")
    }
}

fn ensure_client_state_dir(params: &ClientParameters) -> Outcome<PathBuf> {
    let dir = client_state_dir(params);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("client state dir '{}': {e}", dir.display()))?;
    }
    Ok(dir)
}

fn load_or_create_client_id(path: &Path) -> Outcome<String> {
    if let Ok(s) = fs::read_to_string(path) {
        let line = s.lines().next().unwrap_or("").trim();
        if !line.is_empty() {
            return Ok(line.to_string());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("client_id parent '{}': {e}", parent.display()))?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    fs::write(path, format!("{id}\n")).map_err(|e| format!("write client_id: {e}"))?;
    Ok(id)
}

fn load_acked_generation(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn persist_acked_generation(path: &Path, acked: u64) -> Outcome<()> {
    fs::write(path, acked.to_string()).map_err(|e| format!("persist acked_generation: {e}"))?;
    Ok(())
}

pub(crate) fn load_client_sync_state(
    params: &ClientParameters,
) -> Outcome<Arc<Mutex<ClientSyncState>>> {
    let dir = ensure_client_state_dir(params)?;
    let id_path = dir.join("client_id");
    let ack_path = dir.join("acked_generation");
    let client_id = load_or_create_client_id(&id_path)?;
    let acked_generation = load_acked_generation(&ack_path);
    Ok(Arc::new(Mutex::new(ClientSyncState {
        client_id,
        acked_generation,
        ack_path,
    })))
}

pub(crate) fn attach_client_outbound_basis(
    payload: &mut ipc::Payload,
    sync: &Mutex<ClientSyncState>,
) -> Outcome<()> {
    let s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    payload.client_id.clear();
    payload.client_id.push_str(&s.client_id);
    payload.basis_generation = s.acked_generation;
    payload.head_generation = 0;
    payload.last_writer_client_id.clear();
    Ok(())
}

pub(crate) fn maybe_record_writer_ack(
    sync: &Mutex<ClientSyncState>,
    server_msg: &ipc::Payload,
    local_dirty: &Mutex<HashSet<PathBuf>>,
) -> Outcome<()> {
    let mut s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    if server_msg.last_writer_client_id.is_empty() {
        return Ok(());
    }
    if server_msg.last_writer_client_id == s.client_id
        && server_msg.head_generation > s.acked_generation
    {
        s.acked_generation = server_msg.head_generation;
        persist_acked_generation(&s.ack_path, s.acked_generation)?;
        if let Ok(mut dirty) = local_dirty.lock() {
            dirty.clear();
        }
    }
    Ok(())
}

pub(crate) fn mark_local_dirty(local_dirty: &Mutex<HashSet<PathBuf>>, path: &Path) {
    if let Ok(mut dirty) = local_dirty.lock() {
        let p = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        dirty.insert(p);
    }
}

pub(crate) fn record_pull_acked(
    sync: &Mutex<ClientSyncState>,
    head_generation: u64,
) -> Outcome<()> {
    if head_generation == 0 {
        return Ok(());
    }
    let mut s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    if head_generation > s.acked_generation {
        s.acked_generation = head_generation;
        persist_acked_generation(&s.ack_path, s.acked_generation)?;
    }
    Ok(())
}
