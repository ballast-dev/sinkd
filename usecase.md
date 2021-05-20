# use case

## commands

1. deploy // need to install `sinkd` on server
1. add
1. adduser
1. ls
1. rm
1. rmuser
1. start
1. stop

## config

**sinkd.conf** toml file

1. current owner
1. current users (appendable)
1. anchor_points (folders to sync)
1. store ssh key in the `.ssh/` folder for authentication

## server (harbor)

Listens to changes via rsync daemon, rsync:// on port "8466" TB or "9816" tb 
Accepts many running clients.

## client (barge)

Every barge comes into harbor
