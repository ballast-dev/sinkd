# rsync daemon mode over ssh

There are several common ways to do rsync backups of hosts over ssh:

1. As a non-root user. Upsides: very secure. Downside: cannot back up sensitive files.
2. As root, with a public key. Downsides: Whoever has the private key has full root access to the host being backed up.
3. As root, with a public key and a "forced command". Upsides: Restricts access to the server. Downsides: Requires either careful matching of rsync options (which might change over time), or "validator" scripts. Neither idea sounds very appealing to me.
4. Running rsync in daemon mode on the host being backed up. Upsides: Lots of useful options, like read-only mode, running as a different user if required, server-side excludes/includes, etc. Downsides: Opens up a TCP port that has full filesystem read access and is hard to secure (Ideally you could make the rsync daemon use a unix socket instead, that could be secured by filesystem permissions, but I haven't found a way to do that).

Here is another option that I haven't found much information on, but that I think combines the best aspects of solutions 3 and 4: using rsync daemon mode over ssh.

This is based on a relatively obscure feature mentioned in the `rsync(1)` man page under "USING RSYNC-DAEMON FEATURES VIA A REMOTE-SHELL CONNECTION":

> It is sometimes useful to use various features of an rsync daemon (such
> as named modules) without actually allowing any new socket  connections
> into   a   system  (other  than  what  is  already  required  to  allow
> remote-shell access).  Rsync supports connecting  to  a  host  using  a
> remote  shell  and  then  spawning  a  single-use  "daemon" server that
> expects to read its config file in the home dir  of  the  remote  user.

The part about the configuration file is not correct, [the daemon will look for `/etc/rsyncd.conf` instead][1]. Or maybe it's a compile-time option, but at least on Ubuntu and CentOS the file is looked up in `/etc`. This is not a problem though, as we can override the location via a commandline option.

This allows us to set up the following:

  - A special `rsyncd.conf` file on the host to be backed up that provides a read-only view of the filesystem, with optional includes/excludes (see the rsync man page for details). Example:

        # /root/rsyncd.conf
        uid = root
        gid = root
        log file = /var/log/rsyncd.backup.log
        [home]
            path = /home/
            read only = true
            exclude = lost+found/

  - A key that is restricted to running rsync in daemon mode with the above config file:

        # /root/.ssh/authorized_keys
        command="rsync --config=/root/rsyncd.conf --server --daemon .",no-agent-forwarding,no-port-forwarding,no-pty,no-user-rc,no-X11-forwarding ssh-rsa ...

  - And to restrict the ssh access of the root user to forced commands only:

        # /etc/sshd_config
        [...]
        PermitRootLogin forced-commands-only
        [...]

  - On the host doing the backup, we trigger daemon mode over ssh by using rsync daemon notation for the source combined with the `--rsh=ssh` option, as described in the `rsync` man page:

        # rsync -av --rsh=ssh remote_host::home destination/

    When using [`rsnapshot`][2], the correct `backup` line for `/etc/rsnapshot.conf` is:

        backup	remote_host::home	destination/home/	+rsync_long_args=--rsh=ssh

    Note that you have to specify the directory name again on the destination side, and of course use tabs to separate everything.
    
    When using [`swiftbackup`][3], the format for the `backup` option is:
    
        backup =
            remote_host::home --rsh=ssh

What happens now is that `rsync` connects via ssh to the remote host, where the
forced command starts an rsync daemon for the lifetime of the ssh connection
that does not listen on a TCP port and only talks to the rsync process on the
local side. We won't have to change the `authorized_keys` file if we change
rsync options on the client, and as an added bonus the rsync access is
read-only.

[1]: http://serverfault.com/a/7512
[2]: http://rsnapshot.org/
[3]: https://github.com/trendels/swiftbackup
