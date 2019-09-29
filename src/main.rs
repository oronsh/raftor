use actix::prelude::*;
use std::env;

use raftor::network::Network;

fn main() {
    let sys = System::new("testing");
    let mut net = Network::new();

    let args: Vec<String> = env::args().collect();
    let local_address = args[1].as_str();
    let remote_address = args[2].as_str();

    net.register_node(remote_address);
    net.listen(local_address);

    net.start();

    sys.run();
}
