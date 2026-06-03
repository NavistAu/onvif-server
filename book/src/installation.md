# Installation

## Adding the dependency

```
cargo add onvif-server
```

Or add it manually to your `Cargo.toml`:

```toml
[dependencies]
onvif-server = "0.1.0"
```

## The `discovery` feature

WS-Discovery multicast support is gated behind the optional `discovery` feature,
which pulls in the `socket2` crate for low-level UDP multicast socket control.
Enable it when you want the device to be auto-discoverable on the local network:

```toml
[dependencies]
onvif-server = { version = "0.1.0", features = ["discovery"] }
```

See [WS-Discovery](./discovery.md) for details on what this enables at runtime.

## MSRV

The minimum supported Rust version is **1.85.1** (the toolchain channel pinned in
`rust-toolchain.toml`).
