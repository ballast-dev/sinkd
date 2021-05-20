# Order of Operations
1. create `sinkd rig` to initialize server 
1. setup rsync daemon with modules (outside the LAN domain use ssh)
1. pull excludes from `sinkd.conf` 


# sinkd rig 
1. `echo long_string > /etc/rsyncd.conf`
```
uid = nobody
gid = nobody
use chroot = no
max connections = 4
syslog facility = local5
pid file = /run/rsyncd.pid

[sinkd]
	path = /srv/sinkd
	read only = false
```
2. `sudo mkdir sinkd`
3. `sudo groupadd sinkd` 
4. `sudo chown nobody:sinkd /srv/sinkd`
5. `sudo rsync --daemon`

`rsync --mkpath hello_world.txt localhost::sinkd/some/path/`  
- creates path for the file 