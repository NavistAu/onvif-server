# Services

`onvif-server` exposes five ONVIF service traits. Implement the ones relevant to
your hardware. All trait methods have default implementations that return
`OnvifError::NotImplemented`, which maps to a SOAP fault with the ONVIF
`ter:ActionNotSupported` subcode. Clients see a well-formed fault rather than a
connection error.

## `DeviceService`

**Required.** Handles Device Management operations: device information, system date
and time, capabilities, network interfaces, scopes, and hostname configuration.
Registered with `.device_service(impl)` on the builder. The server mounts it at
`/onvif/device_service`.

## `MediaService`

Handles Media profiles, stream URIs, and snapshot URIs. Registered with
`.media_service(impl)`. When registered, the server mounts it at
`/onvif/media_service` and advertises media capabilities in `GetCapabilities`.

## `PTZService`

Handles PTZ control operations: relative move, absolute move, continuous move,
stop, status, and preset management (get, set, goto, remove). Registered with
`.ptz_service(impl)`. Mounted at `/onvif/ptz_service` when registered.

## `ImagingService`

Handles imaging configuration: brightness, contrast, sharpness, and other imaging
settings. Registered with `.imaging_service(impl)`. Mounted at
`/onvif/imaging_service` when registered.

## `EventService`

Handles event subscriptions and notifications. Registered with
`.event_service(impl)`. Mounted at `/onvif/events_service` when registered.

## Unimplemented methods

Any method not overridden in your trait implementation returns
`Err(OnvifError::NotImplemented)`. The SOAP layer converts this to a
`ter:ActionNotSupported` fault response automatically. You can implement services
incrementally — start with the methods your client actually calls.
