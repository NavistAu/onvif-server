# PTZ Namespace Fix — Design Spec

**Date:** 2026-04-11
**Status:** Approved

## Problem

Frigate's autotracking fails to initialise. The Frigate log shows:

```
zeep.exceptions.Fault: Action not supported: {http://www.onvif.org/ver20/ptz/wsdl}GetConfigurationOptions
```

## Root Cause

`GetServices` in `src/service/device.rs` advertises the PTZ service under
`http://www.onvif.org/ver20/ptz/wsdl`. There is no official `ver20` PTZ namespace
in ONVIF — the canonical PTZ namespace has always been `ver10`.

Frigate uses `onvif-zeep-async`, which maps the namespace from `GetServices` to its
bundled WSDL files. Seeing `ver20`, it loads its bundled `ver20` PTZ WSDL and
generates all PTZ SOAP requests with the `ver20` element namespace.

The soap-server dispatch table is built from the bundled `ptz.wsdl`, whose
`targetNamespace` is `http://www.onvif.org/ver10/ptz/wsdl`. Dispatch is keyed on
full QName (namespace + local name). Every PTZ request from Frigate fails at
route-time before the handler is ever called.

The `PTZServiceHandler` itself is correct — unit tests pass because they call
`handle()` directly, bypassing the soap-server dispatch routing.

## Fix

### 1. `src/service/device.rs` — one-line fix

Change the PTZ namespace in `handle_get_services`:

```
- <tds:Namespace>http://www.onvif.org/ver20/ptz/wsdl</tds:Namespace>
+ <tds:Namespace>http://www.onvif.org/ver10/ptz/wsdl</tds:Namespace>
```

### 2. `tests/frigate_compat.rs` — namespace alignment

All PTZ body elements in this test use `xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"`.
After the fix, Frigate will send `ver10`. Update all PTZ body namespaces in the test
to `ver10` so the test reflects what Frigate actually sends.

### 3. `tests/device_management.rs` — regression assertion

Add an assertion to `device_get_services` verifying the PTZ namespace value is
`ver10/ptz/wsdl`. This catches a regression back to `ver20`.

### 4. `tests/device_management.rs` — HTTP-level PTZ dispatch test

Add an integration test that:
- Starts a full `OnvifServer` on a free port
- POSTs a SOAP `GetConfigurationOptions` request (with `ver10` namespace) to `/onvif/ptz_service`
- Asserts HTTP 200 and a response containing `TranslationSpaceFov`

This exercises the full dispatch pipeline (envelope parse → QName routing →
handler → response) and would have caught the original bug.

## What Does Not Change

- `ptz.wsdl` (already uses `ver10` targetNamespace — correct)
- `PTZServiceHandler` (already correct)
- soap-server dispatch logic (no namespace aliasing needed)
- All other service namespaces in `GetServices` (`ver10` device, `ver10` media,
  `ver20` imaging, `ver10` events — all correct)
