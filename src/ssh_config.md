# To create Passwordless login
```bash
$ ssh-keygen -t rsa # or -t ed25519
$ ssh-copy-id remote_user@remote_IP
$ cat .ssh/authorized_keys      # on server
$ ssh-add                       # on client
```
