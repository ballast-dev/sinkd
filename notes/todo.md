# Order of Operations
1. create `sinkd rig` to initialize server 
1. setup rsync daemon with modules (outside the LAN domain use ssh)
1. pull excludes from `sinkd.conf` 

# sinkd init|rig 
look at [sinkd-rig.sh](../sinkd-rig.sh)

# sinkd stop harbor
_why the echo $1?_ 
```bash
ssh [user@]host << EOF
echo $1 | sudo kill -15 $(cat /run/rsync.pid)
EOF
```

`rsync --mkpath hello_world.txt localhost::sinkd/some/path/`  
- creates path for the file 