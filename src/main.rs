/**
 * -- sinkd
 * 
 * ~~~~~~~~~~~~~~~~~> anchor and deploy <~~~~~~~~~~~~~~~~~~~~
 * 
 * rsync between two directories over specialize port 9816 
 * using mqtt to handle notify messages between folders
 * invoke inotify on linux to retreive folder actions. 
 * Or maybe a boolean since last write. 
 * 
 * run rsync as a daemon underneath the hood. delta algorithm
 * has been proven. Maybe cron job the folder ? once a minute? 
 * use as many threads (thread pool!), maybe multiprocess
 * 
 * `sinkd deploy 192.168.1.1`
 * 1) create folder in user chosen location
 * 2) invoke rsync daemon watching the folder. 
 * 3) have the anchor running forever waiting for connection loop{}
 * 4) vessel (client) will be worked upon, push up to anchor
 * 5) make sure within user space (no passwords needed) except pre known
 */


fn main() {
    println!("Sinkd, bring everything with you and the kitchen sink");

}
