// tests/device_management.rs
// Integration tests for Phase 2: Device Management (DEV-01 through DEV-07)
// Wave 0: stubs compile; #[ignore] removed as each handler is implemented.

#[tokio::test]
#[ignore]
async fn device_get_system_date_and_time() {
    // TODO: start server, send unauthenticated GetSystemDateAndTime SOAP request,
    // assert HTTP 200, assert XML contains tt:UTCDateTime element
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_capabilities_xaddr() {
    // TODO: authenticated GetCapabilities; assert tt:Device/tt:XAddr matches server address
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_services() {
    // TODO: authenticated GetServices; assert tds:Service element present with Namespace + XAddr
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_device_information() {
    // TODO: authenticated GetDeviceInformation with populated DeviceInfo; assert all 5 fields returned
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_scopes() {
    // TODO: authenticated GetScopes; assert Fixed scope URIs present
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_hostname() {
    // TODO: authenticated GetHostname; assert HostnameInformation/Name present
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_get_network_interfaces() {
    // TODO: authenticated GetNetworkInterfaces; assert at least one NetworkInterfaces element
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_auth_valid_credential() {
    // TODO: valid WS-Security UsernameToken digest → HTTP 200 on authenticated operation
    todo!()
}

#[tokio::test]
#[ignore]
async fn device_auth_invalid_credential() {
    // TODO: wrong password → SOAP auth fault response (not HTTP 200)
    todo!()
}
