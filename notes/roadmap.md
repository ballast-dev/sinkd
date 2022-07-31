# Roadmap

## Updating from server
1. First, setup interval checking on client side
> situation to consider, even though interval checking will be less time than pushing updates
> that still leaves the issue with what if an editor has had the contents modified **while open**
> most editors will realize this with git and store a copy in memory before writing to disk
2. Interval checking is wasted cycles, and stateless
3. spinning up mqtt increases dependencies (but might have to be the path forward)
4. **meta-data** could be the way
  - in `.sinkd/meta` create list of top-level-folders with rolling numbers (for updating)
  - every client has a fetch cycle that happens _before_ pushing updates
5. could simply to just a **fetch-cycle** per alloted interval
  - name the config entry `phone_home = 4` or `fetch_changes = 7`

## Dynamic DNS

This could provide a way to browse to the home site of sinkd. "blah.sinkd.co" Could possibly link against.

__Actually__ the best way to go about this is to set up my own DNS on sinkd.co and have the app login to subdomain that brings the user to their files. A user could login into sinkd.co and then sinkd.co will remember the address to their home network.
Maybe it would be beneficial to sub lease hosting to DynDNS or something.

## `rsnapshot`
https://github.com/rsnapshot/rsnapshot
**rsnapshot** could prove to be extremely useful for further extension
leveraging the heavy use of _hard-links_ able to remember deltas across snapshots
- This would allow `sinkd archive|stow` to mark off a time in the cloud as "good"
- initial thoughts are to leave it up to user, with the option of setting a flag in the daemon to "snapshot"


## Version Control
- For **shared** files only
- every change is a commit?

### Weigh in on `rsync --daemon`

Useful setup: https://romain.taprest.fr/posts/tech/backup-nextcloud

Nice tip: https://gist.github.com/trendels/6582e95012f6c7fc6542


## Business
1. name - "Ballast Development"
1. logo - "airplane dropping cargo" 
1. taxes - list business on taxes (schedule C)

## Business Plan
1. product
  - harddrive enclosure custom PCB 
    - 3d printing (prototyping) -> eventually outsource 
  - AWS cloud storage (subscriptions)
  - Phone apps (premium tier)
1. Open Source 
  - sinkd engine 
1. Duties
  - CEO Tony
  - CTO Tobin
  - 