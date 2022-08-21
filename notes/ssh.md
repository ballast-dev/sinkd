# Set up passwordless login via SSH

> https://linuxhint.com/setup_ssh_without_passwords/

- EdDSA Curve25519 was chosen based on this article: https://goteleport.com/blog/comparing-ssh-keys/

## Some notes before starting
1. ensure sshd is up and running `sudo systemctl enable sshd && sudo systemctl start sshd` 
1. ensure port 22 is allowed through the firewall `sudo ufw allow ssh` 


## Procedure
1. *On Local*
1. `ssh-keygen -t ed25519`
1. `ssh-copy-id remote_user@remote_IP`

1. *On Remote*
1. `chmod 600 .ssh/authorized_keys`
1. `chmod 700 .ssh`

1. *On Local*
1. `ssh-add`
    - *In our local machine, we will add the private key to the SSH authentication agent. This will allow us to log into the remote server without having to enter a password every time*

