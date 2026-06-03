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
