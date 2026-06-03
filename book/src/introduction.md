# Introduction

`onvif-server` is an ONVIF **Profile S streaming-core** device server library for
Rust, built on top of the `soap-server` crate. You implement the service traits for
your camera hardware to expose a device that standard ONVIF clients — VMS/NVR
software, Home Assistant, Frigate, python-onvif-zeep, ONVIF Device Manager — can
discover and stream from. It targets the Profile S streaming core, not every ONVIF
operation; see [Operation Coverage](./coverage.md) for the exact support claims.

## ONVIF Profile S coverage

| Service  | Status    |
|----------|-----------|
| Device   | Supported |
| Media    | Supported |
| PTZ      | Supported |
| Imaging  | Supported |
| Events   | Supported |

"Supported" means the service is routed and covers the **Profile S streaming
core** — not every operation. See [Operation Coverage](./coverage.md) for the
exact per-operation breakdown (trait-backed / static / framework / absent, with
default behaviour) and [Capabilities & Limitations](./capabilities.md) for the
crate-level summary.

## License

`onvif-server` is dual-licensed under **MIT OR Apache-2.0** (your choice).
