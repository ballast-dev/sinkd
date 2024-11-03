#!/bin/bash

# first set up ssh keys for passwordless login
printf '\n\n' | ssh-keygen -t ed25519
echo $1 | ssh-copy-id -i ~/.ssh/id_ed25519.pub remote_user@remote_IP
eval $(ssh-agent)
ssh-add ~/.ssh/ed25519
# ssh-add, adds private key to authentication agent allowing passwordless login


# Now need to set up rsync daemon 
# priviledge acces is required for setup 
# password is required but will be gathered from user 
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
