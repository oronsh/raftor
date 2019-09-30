use actix::prelude::*;
use std::env;

use raftor::network::Network;

fn main() {
    let sys = System::new("testing");
    let mut net = Network::new();

    let args: Vec<String> = env::args().collect();
    let local_address = args[1].as_str();

    /// listen on ip and port
    net.listen(local_address);

    /// register peers
    let peers = vec![
        "127.0.0.1:8000",
        "127.0.0.1:8001",
        "127.0.0.1:8002",
    ];

    net.peers(peers);

    net.start();
    sys.run();
}
