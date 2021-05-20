#!/bin/bash
CONNECTIONS=14
GROUP='sinkd'

if [ $# -ne 1 ]; then
    echo Need one password to update server
    exit 1
fi 

ssh tony@hydra << EOF
local HISTSIZE=0  
echo $1 | sudo -Sk mkdir /srv/sinkd
echo $1 | sudo -Sk groupadd sinkd 
echo $1 | sudo -Sk chgrp sinkd /srv/sinkd
echo $1 | sudo -Sk tee /etc/rsyncd.conf << ENDCONF
uid = nobody
gid = nobody
use chroot = no
max connections = $CONNECTIONS
syslog facility = local5
pid file = /run/rsyncd.pid

[sinkd]
    path = /srv/sinkd
    read only = false
    #gid = $GROUP

# HEREDOC is the way 

ENDCONF
echo $1 | sudo -Sk rsync --daemon
EOF
