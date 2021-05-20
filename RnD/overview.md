# sinkd Overview/Brainstorm

## commands

1. `add/anchor` # add file/folder
1. `adduser/hire` # add user? 
1. `ls/parley` # list current watches
1. `rm/brig` # remove watch
1. `rmuser/fire` # remove user?
1. `start/deploy` # start barge daemon
1. `stop/snag` # stop barge daemon
1. `restart/oilskins` # start & stop (updates config)
> 1. `config`

## Package
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

## Development Configuration
- `~/.sinkd/pid` holds running daemon process
- `~/.sinkd/log` to log {info, warnings, errors}
- `~/.sinkd/conf` 
> somehow spawn sinkd daemon on user login
```toml
#        _____       ______ _________                     ____________         
# __________(_)_________  /_______  /  ______________________  __/__(_)______ _
# __  ___/_  /__  __ \_  //_/  __  /   _  ___/  __ \_  __ \_  /_ __  /__  __ `/
# _(__  )_  / _  / / /  ,<  / /_/ /    / /__ / /_/ /  / / /  __/ _  / _  /_/ / 
# /____/ /_/  /_/ /_//_/|_| \__,_/     \___/ \____//_/ /_//_/    /_/  _\__, /  
#                                                                     /____/   

Interval = 5 # in seconds

[[Users]]
name = "bob"
ssh_pubkey = "" # From ssh-keygen on server

[[Users]]
name = "tony"
ssh_pubkey = ""

[[AnchorPoint]]
tld = "top/level/dir" # recursive 
users = ["bob", ..]
excludes = ["dir1", "dir2"]

...
```

   
#### Dynamic DNS

This could provide a way to browse to the home site of sinkd. "blah.sinkd.co" Could possibly link against.

__Actually__ the best way to go about this is to set up my own DNS on sinkd.co and have the app login to subdomain that brings the user to their files. A user could login into sinkd.co and then sinkd.co will remember the address to their home network. 
Maybe it would be beneficial to sub lease hosting to DynDNS or something. 
