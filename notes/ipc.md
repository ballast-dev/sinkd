# Sinkd interprocess communication

- path: `/srv/sinkd/<user>/path...`  
- date: `YYYYMMDD`  
- cycle: `0` _resets upon change of day_ 
## Server vs. Client
| server | client |
| ------ | ------ |
| no config                   | /etc/sinkd.conf and ~/.config/sinkd.conf | 
| /run/sinkd.pid              | /run/sinkd.pid | 
| mkdir /srv/sinkd/ set perms | no /srv/sinkd |
| setup rsync daemon          | no rsync daemon |

## Synchronization
Two options: (will implement _first_ option)
1. **server** the workhorse, all rsync calls will fire off here
    - advantage, only one call at a time (easy to keep state)
    - disadvantage, slow and might bottleneck with tons of clients
1. **clients** offload the work
    - advantage, fast
    - disadvantage, hard to keep state of status

## Client 
1. mqtt subscribe to `sinkd/server`
1. mqtt publish to `sinkd/clients`
1. if cycle = server:cycle send ipc::Payload status=Sinkd
1. if cycle != server:cycle
    1. send status=Behind
    1. wait for server to be status=Sinkd
    1. then send status=Behind  (need to send only once!)
___
- *watch thread*
    - listens to file events
    - filters duplicate events
    - buffers the events and sends them to mqtt thread
- *mqtt thread*
    - if not same cycle number send payload status=Behind
        1. once server has time to process will synchronize
        1. recieve status=Cache from server
        1. cache changes
        1. send status=Updating
        1. server accepts from "this client" initiates rsync call

## Server
1. mqtt subscribe to `sinkd/clients`
1. mqtt publish to `sinkd/server`
1. queue client payloads from `sinkd/clients`
    1. if message is redundant ignore 
1. if client status=Sinkd and same cycle number
    1. run rsync src:client dest:server 
    1. increment cycle number 
1. if client status=Behind
___
1. *mqtt thread*
    - receive packets from clients
    - update broadcast to current status
    - add request to `synch_queue` 
1. *synch thread*
    - process `sync_queue`
    - sets state to `SYNCHING` when processing request
    - once done with all requests set state to `Sinkd` 

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
    > server: needs state, to tell clients when Sinkd 
        1. first msg puts the server in SYNCHING state
        1. second msg is queued 
        1. once all messages are processed 

### roadmap
1. `sinkd status` print out if synchronizing
