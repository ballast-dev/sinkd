# Drawing Board

## Command Line API

| command | alias | function |
| ------- | ----- | -------- |
|`init`    | `rig`      | setup daemon on server tbd... |
|`add`     | `anchor`   | add file/folder |
|`adduser` | `hire`     | add user |
|`ls`      | `parley`   | show current watched files/folders |
|`rm`      | `embay`    | remove file/folder |
|`rmuser`  | `fire`     | remove user? |
|`start`   | `deploy`   | start daemon |
|`stop`    | `drydock`  | stop barge daemon |
|`restart` | `oilskins` | stop then start (updates config) |


# Server vs. Client
| server | client |
| ------ | ------ |
| no config | /etc/sinkd.conf and ~/.config/sinkd.conf | 
| /run/sinkd.pid | /run/sinkd.pid | 
| mkdir /srv/sinkd/ set perms | no /srv/sinkd |
| setup rsync daemon | no rsync daemon |

___
**server** `sinkd init --server` 
  - mkdir /srv/sinkd 
  - chmod 2770 /srv/sinkd (for setgid, not recursive for user permissions to retain)
  <!-- - cd /srv/sinkd/ && umask 5007
  - create systemd unit file with appropriate flags i.e. `sinkd --daemon server` 
  - enable service 
  - start service  -->
  - setup rsync daemon 

**client** `sinkd init --client` (make flag explicit, do not default)
  - create /etc/sinkd.conf 
  - create ~/.config/sinkd.conf 
  - create systemd unit file with appropriate flags i.e. `sinkd --daemon client` 
  - enable service 
  - start service

__Hidden API__ for systemd not for user 
  - `sinkd --daemon server` 
  - `sinkd --daemon client` 



## Configuration Location/Loading 
Config will be loaded from `/etc/sinkd.conf` but also searched in `~/.config/sinkd.conf` for particular user preferences
1. Upon adding and removing files/folders the daemon will be told to reparse it's configuration 
    1. use MQTT
    1. use a signal ... this might be better         

| client side | server side |
| ----------- | ----------- |
| `/absolute/path/to/stuff` | `/srv/sinkd/<user>/absolute/path/to/stuff` |

## Server-Side
- **storage**
    - `/srv/sinkd/` is the "server root"
    - add `sinkd` group and relevent users to that group
    - `/srv/sinkd/<user>/[anchor ...]`
    - `/srv/sinkd/share/[anchor ...]` (multi-user, group permissions)
- **daemon**
    - `/etc/sinkd.conf` (system config)
    - `/run/sinkd.pid` (client side daemon)
    - `/var/log/sinkd.log` (client side logging)


## Client-Side
- `~/.config/sinkd.conf` (user config)
- `/etc/sinkd.conf` (system config)
- `/run/sinkd.pid` (client side daemon)
- `/var/log/sinkd.log` (client side logging)
- _add client logging?_ 

## Packaging
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

