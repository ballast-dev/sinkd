## Command Line API

| command | alias | function |
| ------- | ----- | -------- |
|`init`    | `rig`      | setup daemon on server tbd... |
|`add`     | `anchor`   | add file/folder |
|`adduser` | `hire`     | add user |
|`ls`      | `parley`   | show current watched files/folders |
|`rm`      | `embay`    | remove file/folder |
|`rmuser`  | `fire`     | remove user? |
|`start`   | `deploy`   | start daemon |
|`stop`    | `drydock`  | stop barge daemon |
|`restart` | `oilskins` | stop then start (updates config) |

__Hidden API__ for systemd not for user 
  - `sinkd --daemon server` 
  - `sinkd --daemon client` 


```
# systemctl should use same interface for transparency 
# '*' is for after MVP
sinkd (s)erver start
             stop
             restart
             adduser [USER..]
             -f --config-file
      (c)lient start
             stop
             restart
             add [FILE..]
             -f --config-file
      add (alias for 'client add')
      rm (alias for 'client rm')
      adduser (alias for server adduser)
      rmuser (alias for 'server rmuser')
      ls [--server] (show what files are being tracked)
      **init (should be done by packager)
      --verbose
      --debug
```
