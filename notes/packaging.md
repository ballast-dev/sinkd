## `sinkd init --server` 
  - mkdir /srv/sinkd 
  - chmod 2770 /srv/sinkd (for setgid, not recursive for user permissions to retain)
  - cd /srv/sinkd/ && umask 5007
  - create systemd unit file with appropriate flags
  - enable service 
  - start service 

## `sinkd init --client` (make flag explicit, do not default)
  - create /etc/sinkd.conf 
  - create ~/.config/sinkd.conf 
  - create systemd unit file with appropriate flags i.e. `sinkd --daemon client` 
  - enable service 
  - start service

## Packaging
- (pkgr) mkdir /srv/sinkd
- (pkgr) chmod 2770 /srv/sinkd (for setgid, not recursive for user permissions to retain)
- (pkgr) cd /srv/sinkd/ && umask 5007
- (pkgr) create systemd unit file with appropriate flags
- (pkgr) enable service
- (pkgr) start service >> which calls sinkd::server::start()

With package elevation (_set up permissions correctly_):
    - `/run/sinkd.pid` 
    - `/var/log/sinkd.log`
    - `/etc/sinkd.conf` 
    - `sudo chmod 664` for files
    - `sudo chown sinkd:sinkd` for above files
    - [user] `~/.config/sinkd.conf` 
1. `sudo chmod 2770 /srv/sinkd` with setgid (on server)
1. `sudo useradd -r -g sinkd sinkd` adds user sinkd and assigns group sinkd as well 
1. `newgrp` to login to new group

## Create a service
- create `/usr/lib/systemd/system/sinkd.service` 
```txt
[Unit]
Description=(description of your program)

[Service]
ExecStart=/usr/bin/sinkd deploy

[Install]
WantedBy=multi-user.target
```
## package-manager uninstall 
  - will package up sinkd in a way to smartly remove itself 
