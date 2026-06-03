# Client Setup

Notes for connecting common ONVIF clients to an `onvif-server` device: the URL to
use, auth, which operations the client leans on, and the caveats specific to this
crate's [coverage](./coverage.md).

## Connection basics

- **Service URL:** `http://<host>:<port>/onvif/device_service` (default port 8080).
  `<host>` must match your `advertised_host` and be routable from the client.
- **Auth:** if you called `.auth(user, pass)`, clients need those credentials;
  otherwise leave the client's credentials blank. `GetSystemDateAndTime` is always
  reachable unauthenticated for clock sync.
- **Streams:** the RTSP/snapshot URLs are whatever your `MediaService` returns —
  they are independent of the ONVIF port and must be reachable too.

## Frigate

- Configure the camera with `onvif:` host/port/user/password, plus the RTSP stream
  from `ffmpeg:` inputs (Frigate does not learn the stream URL from ONVIF — you set
  it directly).
- Frigate calls `GetServiceCapabilities` and the PTZ control surface for
  `onvif`-based PTZ. This crate returns `MoveStatus` as an **attribute** (which
  Frigate requires) and implements the PTZ moves/stop/presets via your
  `PTZService`.
- **Caveats:** PTZ buttons do nothing unless you implement `PTZService`. Frigate
  does not depend on this device for motion events (it does its own detection), so
  the lack of event delivery is not a problem for Frigate.

## Home Assistant (ONVIF integration)

- Add the ONVIF integration and enter host, port, username, password. HA calls
  `GetDeviceInformation`, `GetCapabilities`, `GetProfiles`, and the stream/snapshot
  URIs, and sets up PTZ services and an event subscription.
- **Caveats:** HA subscribes to events (the pull-point handshake succeeds), but
  **this device never delivers events**, so no ONVIF binary sensors / motion events
  will fire — use HA-side motion detection instead. Implement
  `get_device_information`, `get_stream_uri`, and `get_snapshot_uri` or HA setup
  steps will fault.

## ONVIF Device Manager (ODM)

- Windows diagnostic tool. Point it at the service URL (or discover on-LAN with the
  `discovery` feature) and enter credentials.
- Good for verifying enumeration: it exercises device info, profiles, video
  configs, PTZ nodes, and the live stream. Operations this crate marks **absent**
  (most `Set*`, imaging options, etc.) show as errors/blank in ODM — that is
  expected, not a bug.

## python-onvif-zeep

- `ONVIFCamera(host, port, user, pass, wsdl_dir)`. zeep builds its client from
  WSDLs, so point `wsdl_dir` at a local ONVIF WSDL set (zeep does not fetch the
  device's `?wsdl`).
- Call only the operations in the [coverage matrix](./coverage.md); others return a
  `ter:ActionNotSupported` fault that zeep raises as a `Fault`. Example:
  `cam.create_devicemgmt_service().GetDeviceInformation()`.

## VLC / RTSP players

- VLC is **not** an ONVIF client. It plays the RTSP stream directly — open the URL
  your `MediaService::get_stream_uri` returns (e.g. `rtsp://<host>:554/stream`).
- Use VLC to confirm the underlying media path works independently of ONVIF
  signalling.
