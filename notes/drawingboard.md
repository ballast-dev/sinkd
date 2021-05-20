# Drawing Board

## commands

| command | alias | function |
| ------- | ----- | -------- |
| `setup`  | `rig`      | setup daemon on server tbd... |
|`add`     | `anchor`   | add file/folder |
|`adduser` | `hire`     | add user |
|`ls`      | `parley`   | show current watched files/folders |
|`rm`      | `embay`    | remove file/folder |
|`rmuser`  | `fire`     | remove user? |
|`start`   | `deploy`   | start daemon |
|`stop`    | `drydock`  | stop barge daemon |
|`restart` | `oilskins` | stop then start (updates config) |

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

## Packaging
With package elevation:
- create `/etc/sinkd.conf` with 664 permissions
- create `/run/sinkd.pid` 
- setup `sinkd` group 
- create `/usr/lib/systemd/system/sinkd.service` 

`sudo useradd -r -g sinkd sinkd` adds user sinkd and assigns group sinkd as well 
`newgrp` to login to new group

```txt
[Unit]
Description=(description of your program)

[Service]
ExecStart=/usr/bin/sinkd deploy

[Install]
WantedBy=multi-user.target
```

#### Dynamic DNS

This could provide a way to browse to the home site of sinkd. "blah.sinkd.co" Could possibly link against.

__Actually__ the best way to go about this is to set up my own DNS on sinkd.co and have the app login to subdomain that brings the user to their files. A user could login into sinkd.co and then sinkd.co will remember the address to their home network. 
Maybe it would be beneficial to sub lease hosting to DynDNS or something. 

# `rsnapshot` 
https://github.com/rsnapshot/rsnapshot  
**rsnapshot** could prove to be extremely useful for further extension  
leveraging the heavy use of _hard-links_ able to remember deltas across snapshots
- This would allow `sinkd archive|stow` to mark off a time in the cloud as "good" 
- initial thoughts are to leave it up to user, with the option of setting a flag in the daemon to "snapshot" 


# Version Control
- For **shared** files only 
- every change is a commit?

# Weigh in on `rsync --daemon`

Useful setup: https://romain.taprest.fr/posts/tech/backup-nextcloud

Nice tip: https://gist.github.com/trendels/6582e95012f6c7fc6542


# Roadmap for updating from server
1. First, setup interval checking on client side 
> situation to consider, even though interval checking will be less time than pushing updates
> that still leaves the issue with what if an editor has had the contents modified **while open**
> most editors will realize this with git and store a copy in memory before writing to disk 
2. Interval checking is wasted cycles, and stateless 
3. spinning up mqtt increases dependencies (but might have to be the path forward)
4. **meta-data** could be the way
  - in `.sinkd/meta` create list of top-level-folders with rolling numbers (for updating)
  - every client has a fetch cycle that happens _before_ pushing updates
5. could simply to just a **fetch-cycle** per alloted interval
  - name the config entry `phone_home = 4` or `fetch_changes = 7`
