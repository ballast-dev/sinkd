//    ____                    
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/   

use crate::protocol::defs;

enum State {
    SYNCHING,
    READY,
}

// `sinkd start` starts up the client daemon
// `sinkd start -s,--server` will start up the server daemon

pub fn start() {

}