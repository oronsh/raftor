use actix::prelude::*;

use raftor::network::Network;

fn main() {
    let sys = System::new("testing");
    let mut net = Network::new();

    net.register_node(0, "127.0.0.1:9000");
    net.listen("127.0.0.1:8000");

    net.start();

    sys.run();
}
