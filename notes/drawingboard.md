# Drawing Board

## Command Line API

| command | alias | function |
| ------- | ----- | -------- |
| `setup`  | `rig`      | setup daemon on server tbd... |
|`add`     | `anchor`   | add file/folder |
|`adduser` | `hire`     | add user |
|`ls`      | `parley`   | show current watched files/folders |
|`rm`      | `embay`    | remove file/folder |
|`rmuser`  | `fire`     | remove user? |

## Configuration Location/Loading 
Config will be loaded from `/etc/sinkd.conf` but also searched in `~/.config/sinkd.conf` for particular user preferences
1. Upon adding and removing files/folders the daemon will be told to reparse it's configuration 
    1. use MQTT
    1. use a signal ... this might be better         

| client side | server side |
| ----------- | ----------- |
| `/absolute/path/to/stuff` | `/srv/sinkd/[user]/absolute/path/to/stuff` |

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
1. `sudo useradd -r -U sinkd` adds user sinkd and assigns group sinkd as well 
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

