# sinkd Overview/Brainstorm

## commands

1. `add` # add file/folder
1. `adduser` # add user? 
1. `ls` # list current watches
1. `rm` # remove watch
1. `rmuser` # remove user?
1. `start` # start barge daemon
1. `stop` # stop barge daemon
> 1. `config`

## New Approach

`sinkd` is a multi-user program, so configurations will be loaded in `/etc/sinkd.conf`

- both server configs and user configs for simplicity
```toml
[SERVER]
paths: [
  "path/one/..",
  "path/two/.."
]
authorized_users: [
  "user_one",
  "user_two",
  "..."
]


[USER.user_one]
interval = 5   # seconds to synchronize

...
```

## /run/sinkd.pid
daemon process to run, store pid 



   
#### Dynamic DNS

This could provide a way to browse to the home site of sinkd. "blah.sinkd.co" Could possibly link against.

__Actually__ the best way to go about this is to set up my own DNS on sinkd.co and have the app login to subdomain that brings the user to their files. A user could login into sinkd.co and then sinkd.co will remember the address to their home network. 
Maybe it would be beneficial to sub lease hosting to DynDNS or something. 
