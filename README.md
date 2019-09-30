# raftor

A wrapper around actix-raft to provide it with network and storage capabilites

TODO:

- [X] Add netowrk actor that establish connections with node clients
- [ ] Refactor network functions to actix message passing interface
- [ ] Initiate cluster discovery
- [ ] Implement actix-raft network
- [ ] Implement actix-raft storage
- [ ] Implement actix-raft metrics
- [ ] Verify raft is working and replicating state across cluster
