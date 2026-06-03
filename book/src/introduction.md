# Introduction

`onvif-server` is a spec-compliant ONVIF Profile S device server library for Rust.
It is built on top of the `soap-server` crate. You implement the service traits for
your camera hardware and receive a fully functional ONVIF-compatible device that any
standard ONVIF client can use — including VMS/NVR software, Home Assistant, Frigate,
python-onvif-zeep, and ONVIF Device Manager.

## ONVIF Profile S coverage

| Service  | Status    |
|----------|-----------|
| Device   | Supported |
| Media    | Supported |
| PTZ      | Supported |
| Imaging  | Supported |
| Events   | Supported |

## License

`onvif-server` is dual-licensed under **MIT OR Apache-2.0** (your choice).
