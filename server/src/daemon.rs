//! Unix/Windows daemon process entry for the server role.

#[cfg(windows)]
use sinkd_core::parameters::DaemonType;
use sinkd_core::{ipc, outcome::Outcome, shiplog};

use crate::{parameters::ServerParameters, server};

pub fn spawn(params: &ServerParameters) -> Outcome<()> {
    let params = params.clone();
    #[cfg(unix)]
    {
        ipc::unix::daemon(move || {
            shiplog::init(&params.shared)?;
            server::init(&params)
        })
    }
    #[cfg(windows)]
    {
        match params.shared.daemon_type {
            DaemonType::WindowsServer => {
                ipc::windows::redirect_stdio_to_null()?;
                shiplog::init(&params.shared)?;
                server::init(&params)
            }
            DaemonType::UnixServer => ipc::windows::daemon().map(|_| ()),
            _ => sinkd_core::bad!("unexpected daemon type for server"),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = params;
        Ok(())
    }
}
