# raftor

Distributed websocket chat

this is an experimental project that distribute websocket connections
over a cluster.

This project is built on top of
[actix](https://github.com/actix/actix) and
[actix-raft](https://github.com/railgun-rs/actix-raft)

## How to run

In `Config.toml` list your nodes under `[[nodes]]`

Every node has a `private_addr` and a `public_addr`

`private_addr` is the network address used for internal communication

and `public_addr` is the network address exposed to the world.

`cargo run CLUSTER_ADDRESS APP_ADDRESS PUBLIC_ADDRESS`

Run in single node
`cargo run 127.0.0.1:8000 127.0.0.1:9000 127.0.0.1:8080`

Run cluster

```
cargo run 127.0.0.1:8000 127.0.0.1:9000 127.0.0.1:8080

cargo run 127.0.0.1:8001 127.0.0.1:9001 127.0.0.1:8081

cargo run 127.0.0.1:8002 127.0.0.1:9002 127.0.0.1:8082
```

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
- [ ] Building js client
