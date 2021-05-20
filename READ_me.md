## //////////////////////\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
## ----------- R O U G H  D R A F T ---------------
## \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\//////////////////////

# Sinkd
##### Everything and the kitchen sink
### deployable cloud, when you want, where you want

Intro:

Do you want to host your files on your own server? We do too. Sinkd is a deployable cloud just give it two folder paths (local and remote) and you are good to go! Just make sure you have access to both machines (write permissions) and **drop anchor**

1. Script will start

2. Request from user `~/path/to/local` to `user@server.tld:~/path/to/remote`

3. Execute python sinkd.py

  * create `.sinkd/` directory inside `~/path/to/local`

    * folder will hold configs and folder locations (for multiple folders)

    * folder locations will be `ln -s` symbolically linked to save space

  * ```cpp
  while(running){
      if ( inotify() ){
          `rsync -vrt ~/path/to/local --rsh==ssh user@server.tld:~/path/to/remote/`
      }
  }
  ```

4. Update to daemon -> incorporate this into `init.d/` daemons that initialize at boot

5. Secure Shell and Secure File Transfer Protocol for Security

___
`rsync` will be the transfer lib

`inotify` is the kernel call to notify the OS of changes to folders
