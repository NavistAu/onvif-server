# Installation

## Adding the dependency

```sh
cargo add onvif-server
```

## The `discovery` feature

WS-Discovery multicast support is gated behind the optional `discovery` feature,
which pulls in the `socket2` crate for low-level UDP multicast socket control.
Enable it when you want the device to be auto-discoverable on the local network:

```sh
cargo add onvif-server --features discovery
```

See [WS-Discovery](./discovery.md) for details on what this enables at runtime.

## MSRV

The minimum supported Rust version is the `rust-version` declared in the crate's
`Cargo.toml` (also shown on the crate's [crates.io page](https://crates.io/crates/onvif-server)).
