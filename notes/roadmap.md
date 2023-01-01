# Roadmap

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


## Virtual file system mounting
Could be useful

## Brad's thoughts
1. Install hook, to install environment
1. Use X server in docker to run gui apps
1. take everything with you, environment/gui/apps/dotfiles (this is it!)