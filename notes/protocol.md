# Sinkd protocol
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct MsgUpdate {
    user: String,
    path: String,
    date: String,
    cycle: u16,
}

enum State {
    SYNCHING,
    READY,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgStatus {
    date: String,
    cycle: u16,
}
```

path: `/srv/sinkd/<user>/path...`  
date: `YYYYMMDD`  
cycle: `0` _resets upon change of day_ 

## Synchronization
Two options: (will implement first option)
1. **server** the workhorse, all rsync calls will fire off here
    - advantage, only one call at a time (easy to keep state)
    - disadvantage, slow and might bottleneck with tons of clients
1. **clients** offload the work
    - advantage, fast
    - disadvantage, hard to keep state of status

## Client 
1. *polling thread* 
    - listen to broadcasts from server
    - update internal status 
1. *sinkd thread*
    - wait for inotify events
    - push packet to server 
    - update internal status to reflect current status

## Server
1. *listening thread*
    - receive packets from clients
    - update broadcast to current status
    - add request to `synch_queue` 
1. *synching thread*
    - process `sync_queue`
    - sets state to `SYNCHING` when processing request
    - once done with all requests set state to `READY` 
1. *broadcast thread*
    - push out messsages with current status 
    - interval (every 5 secs) of status

## Order of operations
> file event
1. client: check if up to date from server
    - if not poll for MsgStatus from server
    - if out of date, update first
        - how to determine what is right? 
        - setup tmp file tree to save off current working state `<tree>/.sinkd/` of affected folder
        - then copy the rest into place after 
        - delete `<tree>/.sinkd/` 
1. client: send MsgUpdate
1. client: update internal status to reflect last sent MsgUpdate
1. server: receive MsgUpdate
1. server: `rsync` the given path relative to user `/srv/sinkd/<user>/path...`
1. server: broadcasts status every 5 secs

### Conflicts
1. 2 MsgUpdate are recieved microseconds apart 
    1. **how to resolve?** 
    1. server: listening thread will only process one at a time
    1. server: _first come, first serve_ approach 
    1. client: will query state from server 
    > server: needs state, to tell clients when ready 
        1. first msg puts the server in SYNCHING state
        1. second msg is queued 
        1. once all messages are processed 

### roadmap
1. `sinkd status` print out if synchronizing
