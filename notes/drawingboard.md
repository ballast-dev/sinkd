# Drawing Board

## Configuration Location/Loading 
Config will be loaded from `/etc/sinkd.conf` but also searched in `~/.config/sinkd.conf` for particular user preferences
1. Upon adding and removing files/folders the daemon will be told to reparse it's configuration 
    1. use MQTT
    1. use a signal ... this might be better   

### Considering removal of `add` and `rm` commands 
This is in favor for a configuration first concept. When user modifies `/etc/sinkd.conf` or the `~/.config/sinkd` file then the program will reparse the contents and use them.       

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
