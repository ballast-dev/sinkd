use clap::parser::ValuesRef;
use log::{info, warn};
use std::fs;

use crate::{
    config::{self, SysConfig},
    ipc,
    outcome::Outcome,
    parameters::ClientParameters,
};

fn notify_reload() {
    if let Err(e) = ipc::publish_config_reload_signal() {
        warn!("config updated but could not publish reload notification over Zenoh: {e}");
    }
}

/// `--share` updates system-config anchors; bare PATH arguments update each resolved user config.
pub fn add(params: &ClientParameters, share_paths: &[&String], user_paths: &[&String]) -> Outcome<()> {
    if share_paths.is_empty() && user_paths.is_empty() {
        return bad!("add: supply at least one --share and/or PATH");
    }

    let sys_path = params.system_config.as_ref().as_path();

    if !share_paths.is_empty() {
        let mut sys = config::load_system_config_file(sys_path)?;
        let anchors = sys.anchors.get_or_insert_with(Vec::new);
        for p in share_paths {
            let resolved = config::resolve(p)?;
            if anchors.iter().any(|a| a.path == resolved) {
                continue;
            }
            anchors.push(config::Anchor::with_path(resolved));
        }
        config::save_system_config_file(sys_path, &sys)?;
        info!("updated system config {}", sys_path.display());
    }

    if !user_paths.is_empty() {
        if params.user_configs.is_empty() {
            return bad!(
                "no user config files resolved; use --usr-cfg or create ~/.config/sinkd/sinkd.conf"
            );
        }
        for user_path in params.user_configs.iter() {
            let mut usr = config::load_user_config_file(user_path.as_path())?;
            for p in user_paths {
                let resolved = config::resolve(p)?;
                if usr.anchors.iter().any(|a| a.path == resolved) {
                    continue;
                }
                usr.anchors.push(config::Anchor::with_path(resolved));
            }
            config::save_user_config_file(user_path.as_path(), &usr)?;
            info!("updated user config {}", user_path.display());
        }
    }

    notify_reload();
    Ok(())
}

pub fn remove(
    params: &ClientParameters,
    share_paths: &[&String],
    user_paths: &[&String],
) -> Outcome<()> {
    if share_paths.is_empty() && user_paths.is_empty() {
        return bad!("remove: supply at least one --share and/or PATH");
    }

    let sys_path = params.system_config.as_ref().as_path();

    if !share_paths.is_empty() {
        let mut sys = config::load_system_config_file(sys_path)?;
        if let Some(anchors) = sys.anchors.as_mut() {
            for p in share_paths {
                let resolved = config::resolve(p)?;
                anchors.retain(|a| a.path != resolved);
            }
        }
        config::save_system_config_file(sys_path, &sys)?;
        info!("updated system config {}", sys_path.display());
    }

    if !user_paths.is_empty() {
        if params.user_configs.is_empty() {
            return bad!(
                "no user config files resolved; use --usr-cfg or create ~/.config/sinkd/sinkd.conf"
            );
        }
        for user_path in params.user_configs.iter() {
            let mut usr = config::load_user_config_file(user_path.as_path())?;
            for p in user_paths {
                let resolved = config::resolve(p)?;
                usr.anchors.retain(|a| a.path != resolved);
            }
            config::save_user_config_file(user_path.as_path(), &usr)?;
            info!("updated user config {}", user_path.display());
        }
    }

    notify_reload();
    Ok(())
}

pub fn adduser(params: &ClientParameters, users: Option<ValuesRef<String>>) -> Outcome<()> {
    let Some(users) = users else {
        return bad!("no user(s) were given!");
    };
    let sys_path = params.system_config.as_ref().as_path();
    let mut sys: SysConfig = config::load_system_config_file(sys_path)?;
    for user in users {
        if !sys.users.iter().any(|u| u == user.as_str()) {
            sys.users.push(user.as_str().to_string());
        }
    }
    config::save_system_config_file(sys_path, &sys)?;
    info!("updated system config {}", sys_path.display());
    notify_reload();
    Ok(())
}

pub fn rmuser(params: &ClientParameters, users: Option<ValuesRef<String>>) -> Outcome<()> {
    let Some(users) = users else {
        return bad!("no user(s) were given!");
    };
    let sys_path = params.system_config.as_ref().as_path();
    let mut sys: SysConfig = config::load_system_config_file(sys_path)?;
    for user in users {
        sys.users.retain(|u| u != user.as_str());
    }
    config::save_system_config_file(sys_path, &sys)?;
    info!("updated system config {}", sys_path.display());
    notify_reload();
    Ok(())
}

pub fn list(
    params: &ClientParameters,
    paths: Option<Vec<&String>>,
    list_server: bool,
) -> Outcome<()> {
    if list_server {
        println!(
            "listing server-side paths is not implemented; inspect the server sync root (e.g. /srv/sinkd on Linux)."
        );
        return Ok(());
    }

    let (_addr, inode_map) = config::get(params)?;
    let mut keys: Vec<_> = inode_map.keys().cloned().collect();
    keys.sort();

    if let Some(filter) = paths {
        if filter.is_empty() {
            return bad!("no paths were given!");
        }
        let resolved: Vec<std::path::PathBuf> = filter
            .iter()
            .map(|p| config::resolve(p))
            .collect::<Result<_, _>>()?;
        for k in keys {
            if resolved.iter().any(|root| k.starts_with(root)) {
                println!("{}", k.display());
            }
        }
    } else {
        for k in keys {
            println!("{}", k.display());
        }
    }
    Ok(())
}

pub fn log(params: &ClientParameters) -> Outcome<()> {
    let data = fs::read_to_string(&params.shared.log_path).map_err(|e| {
        format!(
            "couldn't read log file {}: {e}",
            params.shared.log_path.display()
        )
    })?;
    print!("{data}");
    Ok(())
}
