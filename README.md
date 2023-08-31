# bevy_gaff (work in progress)

bevy_gaff is an attempt at making a networked (p2p), cross-platform physics
simulation using rollback netcode.

It synchronizes only user input and relies on determinism for the simulation to
stay in sync.

It also serves as kind of a showcase/demo of the following rust gamedev crates:

- [Bevy](https://bevyengine.org): game engine
- [GGRS](https://github.com/gschup/ggrs): rollback netcode
- [bevy_xpbd](https://github.com/Jondolf/bevy_xpbd): physics engine
- [Matchbox](https://github.com/johanhelsing/matchbox): p2p networking

On network delays physics state is predicted ahead of time, assuming other
players input will stay the same as the last frame. If conflicting input is
received later, an older state is restored, and the physics simulation is
replayed using the updated input.

This means the simulation stays responsive, and you will always see the result
of your own input immediately regardless of ping to other players.

Input is also sent directly between players (p2p, no intermediate server) even
when running on WASM,  this means that if the players are located closely,
mis-predictions will be rare and the latency barely noticeable.

## Running it

Install and run matchbox_server

```shell
cargo install matchbox_server
matchbox_server
```

...and run two instances of the "game":

```shell
cargo run
```

The example can also be run "single-player":

```shell
cargo run --players 1
```

Or with any other number of players

## Issues

- [ ] simulation occasionally desyncs on rollbacks locally

## Relevant links

- [Gaffer On Games - Introduction to networked physics](https://gafferongames.com/post/introduction_to_networked_physics/)
- https://github.com/bitshifter/glam-rs/discussions/388