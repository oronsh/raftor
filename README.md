# raftor

Distributed websocket chat

TODO:

- [X] Initiate cluster discovery
- [X] Implement actix-raft
- [X] Verify raft is working and replicating state across cluster
- [X] Refactor hash_ring to Arc in order to reduce complexity
- [ ] Error handling
- [X] Fix rejoin bug
- [ ] Rehash entities when hash ring topology changes
