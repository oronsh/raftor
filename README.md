# raftor

Distributed websocket chat
this is an experimental project that distribute websocket connections
over a cluster.

This project is built on top of
[actix](https://github.com/actix/actix) and
[actix-raft](https://github.com/railgun-rs/actix-raft)

## How to run

In `Config.toml` list your nodes under `[[node]]`
Every node has a `private_addr` and a `public_addr`
`private_addr` is the network address used for internal communication
and `public_addr` is the network address exposed to the world.

## API

Create room
`/room/<Name>`

TODO:

- [X] Initiate cluster discovery
- [X] Implement actix-raft
- [X] Verify raft is working and replicating state across cluster
- [X] Refactor hash_ring to Arc in order to reduce complexity
- [ ] Error handling
- [X] Fix rejoin bug
- [ ] Rehash entities when hash ring topology changes
