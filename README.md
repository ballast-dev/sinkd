
### > R O U G H  D R A F T <

# Sinkd
##### Everything and the kitchen sink
### deployable cloud, when you want, where you want
___
Intro:

Do you want to host your files on your own server?
We do too. Sinkd is a deployable cloud!
Just give it two folder paths (local and remote) and **drop anchor**
                              _-_
                             |(_)|
                              |||
                              |||
                              |||
                              |||
                              |||
                        ^     |^|     ^
                      < * >   <=>   < * >
                       | |    |||    | |
                        \ \__/ | \__/ /
                          \,__.|.__,/
                              (_)
```
           
1. Script will start

2. Request from user `~/path/to/local` to `user@server.tld:~/path/to/remote`

3. Execute python sinkd.py

  * create `.sinkd/` directory inside `~/path/to/local`

    * folder will hold configs and folder locations (for multiple folders)

    * folder locations will be `ln -s` symbolically linked to save space

  ```cpp
  while(running){
      if ( inotify() ){
          `rsync -vrt ~/path/to/local --rsh==ssh user@server.tld:~/path/to/remote/`
      }
  }
  ```

4. Update to daemon -> incorporate this into `init.d/` daemons that initialize at boot

5. Secure Shell and Secure File Transfer Protocol for Security

___
## Other thoughts
#### Model View Controller Design
Restructure the `web/` side of the app to be modular
* view = html/css _separate folder?_
* model = `db/username` and mysql database
* controller = `main.php` **user event driven**

#### Daemon
* `rsync` will be the transfer lib
* `inotify` is the kernel call to notify the OS of changes to folders
