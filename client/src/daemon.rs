//! Unix/Windows daemon process entry for the client role.

#[cfg(windows)]
use sinkd_core::parameters::DaemonType;
use sinkd_core::{ipc, outcome::Outcome, shiplog};

use crate::{client, params::ClientParameters};

pub fn spawn(params: &ClientParameters) -> Outcome<()> {
    let params = params.clone();
    #[cfg(unix)]
    {
        ipc::unix::daemon(move || {
            shiplog::init(&params.shared)?;
            client::init(&params)
        })
    }
    #[cfg(windows)]
    {
        match params.shared.daemon_type {
            DaemonType::WindowsClient => {
                ipc::windows::redirect_stdio_to_null()?;
                shiplog::init(&params.shared)?;
                client::init(&params)
            }
            DaemonType::UnixClient => ipc::windows::daemon().map(|_| ()),
            _ => sinkd_core::bad!("unexpected daemon type for client"),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = params;
        Ok(())
    }
}
