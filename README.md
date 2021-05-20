# ![image](sinkd-logo.png)

_**Everything and the kitchen sink**_  
> Deployable cloud, when you want, where you want  
  ... sinkd, main font should be `moonhouse` all low case

## Why makes sinkd different from other cloud providers

Do you want to host your files on your own server?
We do too. `sinkd` is a deployable cloud!

### Harbor (Server)

> Harbor is where all your files are located, every ship comes into port

1. make sure ssh keys are set up, manual or automatic?
1. create folder on server machine ~/sinkd/
1. two subfolders (on server)
    - `harbor/` root folder location
    - `.git/` (feature enhancement) for versioning and back up (_git-lfs needed_)
1. invoke rsync daemon on port 9816 (tb).
1. incorporate this into `init.d/` to initialize at boot  

__Feature Enhancement__  
To aide in set up
    - GUI
    - TUI (ncurses lib in rust)

### Barge (Client)

- Need to setup a configuration file that is parsed upon invocation
- name the file sinkd.json (_checkout yaml-rust_)

1. user specifies location of `sinkd` folder.
1. `sinkd` will run user-wide, and operate on loaded directories within it's configuration
1. upon "anchoring" the folder sinkd will become aware and watch for events
1. upon event change, invoke rsync
1. maybe set a config value to limit file syncing to a known cycle ( 1sec - 1hr )

## Command Line Interface

- `sinkd deploy IP` creates harbor on server
- `sinkd anchor DIRECTORY` creates DIRECTORY on harbor (server file location)
  - loads DIRECTORY in sinkd.json (top-level)
  - possibility of multiple directories inside harbor folder
- `sinkd start` starts daemon
- `sinkd stop` stops daemon
- `sinkd restart` restarts daemon

## Will use SSH key-based authentication

For security and authentication, use an ssh tunnel for file transfers

### Other thoughts

#### Model View Controller Design

Restructure the `web/` side of the app to be modular

- view = html/css _separate folder?_
- model = `db/username` and mysql database
- controller = `main.php` **user event driven**
