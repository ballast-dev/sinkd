# sinkd Overview/Brainstorm

## commands

1. `add` # add folder
1. `adduser` # add user? 
1. `ls` # list current watches
1. `rm` # remove watch
1. `rmuser` # remove user?
1. `start` # start barge daemon
1. `stop` # stop barge daemon

The following will be invoked on the "server" computer aka **harbor**

1. `harbor --dock` # to generate daemon on "server"
1. `harbor 

## New Approach

`sinkd` is a multi-user program, so configurations will be loaded in /etc

- **Package** will have `/etc/sinkd/barge.conf` and `/etc/sinkd/harbor.conf` (TOML files)
- `sinkd harbor` will control everything in the _harbor_
  - harbor control will be specific to machine running 
- `sinkd` will default to barge controls

### barge.conf

1. anchor_points (folders to sync)
1. store ssh key in the `.ssh/` folder for authentication
> NOTE: no need to store user permissions each 'barge' will run per user

## harbor.conf

1. users 
1. anchor_points 
  - with user authentication to r/w 
  - single point can be shared by multiple users (how to handle this?)


## Dynamic DNS

This could provide a way to browse to the home site of sinkd. "blah.sinkd.co" Could possibly link against.

__Actually__ the best way to go about this is to set up my own DNS on sinkd.co and have the app login to subdomain that brings the user to their files. A user could login into sinkd.co and then sinkd.co will remember the address to their home network. 
Maybe it would be beneficial to sub lease hosting to DynDNS or something. 