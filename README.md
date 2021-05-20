# ![image](sinkd-logo.png)

_**Everything and the kitchen sink**_  

## Deployable Cloud

_True Privacy_  
I believe your files should stay with you **always**.
No third party eyes, no privacy policies, no tradeoffs. 
Given the pleathora of cloud providers and the frequent 
data breaches of large companies I created `sinkd` to 
give the power back to the user. 

1. Wraps `rsync` into a user friendly way
1. Physical access to your files
1. Granular permissions per user  
1. Data restore, backup utility

## What about [rclone](https://rclone.org/)?

`rclone` is a fantastic application written in [Go](https://golang.org/). `sinkd` is written in [Rust](https://www.rust-lang.org) which is superior to **Go** in many ways. Also the goal of `rclone` is to use cloud data providers from the command line. The goal of `sinkd` is to **not** to use any cloud provider and keep everything _in house_. If the user wishes to allow access to the web that will be allowed but not enabled by default. 

### Future - Feature Enhancements

- enable `btrfs` on a virtual mount for added integrity
- add encryption to file system
- access from the interwebs 