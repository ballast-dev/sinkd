// need to make a sinkd group for permissions
/* -----------
* H A R B O R
* ----------- 
*/
pub struct Harbor {
    config: String  // parsed yaml from /etc/sinkd.conf
}

impl Harbor {
    pub fn init_rsyncd() {
        // initialize the rsync daemon 
        // `rsync --daemon`
        // read the special config shipped with sinkd
        // `sinkd deploy 10.0.0.1` should call this function

        // the directory to store sinkd data is /srv/sinkd
    }

    fn parse_config() -> bool {
        // make sure to have permission to read config file
        return true
    }

    pub fn start() {

    }
}
