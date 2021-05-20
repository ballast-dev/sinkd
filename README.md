
### > R O U G H  D R A F T <

# Sinkd
##### Everything and the kitchen sink
### deployable cloud, when you want, where you want
___
Intro:

Do you want to host your files on your own server?
We do too. Sinkd is a deployable cloud!
Just give it two folder paths (local and remote) and **drop anchor**

**~~~~~~~~~~~~~~~~~> anchor and deploy <~~~~~~~~~~~~~~~~~~~~**

rsync between two directories over specialize port 9816 
using mqtt to handle notify messages between folders
invoke inotify on linux to retreive folder actions. 
Or maybe a boolean since last write. 

run rsync as a daemon underneath the hood. delta algorithm
has been proven. Maybe cron job the folder ? once a minute? 
use as many threads (thread pool!), maybe multiprocess

`sinkd deploy 192.168.1.1`
1) create folder in user chosen location
2) invoke rsync daemon watching the folder. 
3) have the anchor running forever waiting for connection loop{}
4) vessel (client) will be worked upon, push up to anchor
5) make sure within user space (no passwords needed) except pre known


first thing sinkd needs to know about two folder locations
I think I need two binaries 'daemon'=>'anchor' and 'client'=>'barge'
anchor should run as the server on another machine (asynchronous) 

Update to daemon -> incorporate this into `init.d/` daemons that initialize at boot

## Anchor/Daemon
  - this will comand line driven (GUI will come later)
  - rsync can be invoked as a daemon, wrap this up to be a running job in rust
  - set up folder for sinkd, (user defined)  
  - `inotify` is the kernel call to notify the OS of changes to folders

## Barge/Client
  - loads up folder to hook into the daemon running on the anchor point 
  - changes will be invoked on the barge, then transferred to anchor
  - ...
___
## Other thoughts
#### Model View Controller Design
Restructure the `web/` side of the app to be modular
* view = html/css _separate folder?_
* model = `db/username` and mysql database
* controller = `main.php` **user event driven**
