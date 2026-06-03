# WS-Security

## Enabling authentication

Call `.auth(username, password)` on the builder to enable WS-Security
UsernameToken digest authentication:

```rust,no_run
OnvifServer::builder()
    .port(8080)
    .device_service(MyCamera)
    .auth("admin", "password")
    .build()
    .expect("build failed")
    .run()
    .await
    .expect("server error");
```

When `.auth()` is called, every SOAP request must include a valid WS-Security
`UsernameToken` header with a matching username and password digest. Requests
without a valid token receive a SOAP authentication fault.

When `.auth()` is **not** called, the server runs unauthenticated. All operations
are accessible without credentials.

## Auth bypass: `GetSystemDateAndTime`

`GetSystemDateAndTime` is automatically exempt from authentication regardless of
whether `.auth()` is called. This is required by the ONVIF specification: clients
must be able to retrieve the device's system time before they have valid credentials,
because the WS-Security digest is time-sensitive and requires clock synchronisation.

No additional configuration is needed — the exemption is pre-registered by the
builder at construction time.

## The clock-sync flow

ONVIF UsernameToken digest authentication is **time-sensitive**: the digest is
`Base64(SHA-1(Nonce + Created + Password))`, and the server rejects a `Created`
timestamp outside a ±300 s window (and replays of a `Nonce` within that window).
A client whose clock is skewed from the device by more than ~5 minutes cannot
authenticate. The standard handshake works around this:

1. **Unauthenticated `GetSystemDateAndTime`** — the client reads the device clock
   (this operation is auth-exempt, above).
2. The client computes its offset from the device and uses the device's time as
   the basis for the `Created` timestamp in step 3.
3. **Digest-authenticated calls** — every subsequent request carries a
   `<wsse:Security>` `UsernameToken` whose `Created`/`Nonce` are accepted because
   they fall inside the device's freshness window.

This is why `GetSystemDateAndTime` must remain reachable without credentials, and
why a device with a badly wrong clock will appear to reject correct passwords.

## A UsernameToken request

A digest-authenticated request carries this header (the client computes the
`Password` digest from a fresh `Nonce` and `Created`):

```xml
<s:Header>
  <wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
                 xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
    <wsse:UsernameToken>
      <wsse:Username>admin</wsse:Username>
      <wsse:Password Type=".../username-token-profile-1.0#PasswordDigest">
        9kFw...base64-digest...=</wsse:Password>
      <wsse:Nonce>LKqI...base64-nonce...=</wsse:Nonce>
      <wsu:Created>2026-06-03T08:00:00Z</wsu:Created>
    </wsse:UsernameToken>
  </wsse:Security>
</s:Header>
```

`PasswordText` (plaintext `<wsse:Password>`) is also accepted. The digest and
replay/freshness mechanics are implemented in the underlying `soap-server` crate —
see its [WS-Security page](https://navistau.github.io/soap-server/ws-security.html)
for the exact algorithm, the ±300 s windows, and multi-process caveats.

## Authentication failure

A request to a protected operation with a missing or invalid `<wsse:Security>`
header gets a SOAP `Sender` (SOAP 1.1: `Client`) fault, in the same SOAP version
as the request:

```xml
<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope">
  <env:Body>
    <env:Fault>
      <env:Code><env:Value>env:Sender</env:Value></env:Code>
      <env:Reason><env:Text xml:lang="en">WS-Security header required but not provided</env:Text></env:Reason>
    </env:Fault>
  </env:Body>
</env:Envelope>
```
