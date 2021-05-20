# Drawing Board

## commands

| command | alias | function |
| ------- | ----- | -------- |
| `init`   | `rig`      | setup daemon on server tbd... |
|`add`     | `anchor`   | add file/folder |
|`adduser` | `hire`     | add user |
|`ls`      | `parley`   | show current watched files/folders |
|`rm`      | `brig`     | remove file/folder |
|`rmuser`  | `fire`     | remove user? |
|`start`   | `deploy`   | start daemon |
|`stop`    | `snag`     | stop barge daemon |
|`restart` | `oilskins` | stop then start (updates config) |

## Configuration Location/Loading 
Config will be loaded from `/etc/sinkd.conf` but also searched in `~/.config/sinkd.conf` for particular user preferences
1. Upon adding and removing files/folders the daemon will be told to reparse it's configuration 
    1. use MQTT
    1. use a signal ... this might be better         

| client side | server side |
| ----------- | ----------- |
| `/absolute/path/to/stuff` | `/srv/sinkd/[user]/absolute/path/to/stuff` |

## On Server
- `/srv/sinkd/` is the "server root"
- add `sinkd` group and relevent users to that group

## On Client Side
- `~/.sinkd/pid` holds running daemon process
- `~/.sinkd/log` to log {info, warnings, errors}


## Packaging
With package elevation:
- create `/etc/sinkd.conf` with 664 permissions
- create `/run/sinkd.pid` 
- setup `sinkd` group 
- create `/usr/lib/systemd/system/sinkd.service` 

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
