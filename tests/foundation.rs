use onvif_server::{
    EmbeddedWsdlLoader, OnvifError, OnvifServer, WsdlLoader, PROFILE_TOKEN, PTZ_CONFIG_TOKEN,
    PTZ_NODE_TOKEN, TRANSLATION_SPACE_FOV, VIDEO_SOURCE_TOKEN,
};

#[test]
fn test_not_implemented_fault_has_ter_namespace() {
    let fault = OnvifError::NotImplemented.into_soap_fault();
    let detail = fault
        .detail
        .expect("detail must be present for ONVIF faults");
    assert!(
        detail.contains(r#"xmlns:ter="http://www.onvif.org/ver10/error""#),
        "detail must contain ONVIF ter namespace declaration, got: {detail}"
    );
}

#[test]
fn test_token_constants_defined() {
    assert!(!PROFILE_TOKEN.is_empty(), "PROFILE_TOKEN must not be empty");
    assert!(
        !VIDEO_SOURCE_TOKEN.is_empty(),
        "VIDEO_SOURCE_TOKEN must not be empty"
    );
    assert!(
        !PTZ_NODE_TOKEN.is_empty(),
        "PTZ_NODE_TOKEN must not be empty"
    );
    assert!(
        !PTZ_CONFIG_TOKEN.is_empty(),
        "PTZ_CONFIG_TOKEN must not be empty"
    );
    assert!(
        !TRANSLATION_SPACE_FOV.is_empty(),
        "TRANSLATION_SPACE_FOV must not be empty"
    );
    assert!(
        TRANSLATION_SPACE_FOV.contains("TranslationSpaceFov"),
        "TRANSLATION_SPACE_FOV must contain 'TranslationSpaceFov', got: {TRANSLATION_SPACE_FOV}"
    );
}

// Stubs for future plans — implemented in later plans

#[test]
fn test_embedded_wsdl_loader() {
    let loader = EmbeddedWsdlLoader;
    let bytes = loader
        .load("devicemgmt.wsdl")
        .expect("devicemgmt.wsdl must load");
    assert!(!bytes.is_empty());
    let bytes = loader.load("media.wsdl").expect("media.wsdl must load");
    assert!(!bytes.is_empty());
    let bytes = loader.load("ptz.wsdl").expect("ptz.wsdl must load");
    assert!(!bytes.is_empty());
}

#[tokio::test]
async fn test_not_implemented_returns_error() {
    use onvif_server::{DeviceService, OnvifError};
    struct StubDevice;
    #[async_trait::async_trait]
    impl DeviceService for StubDevice {}
    let svc = StubDevice;
    // get_device_information has no sensible default — must return NotImplemented
    let result = svc.get_device_information().await;
    assert!(matches!(result, Err(OnvifError::NotImplemented)));
    // get_system_date_and_time defaults to Ok(Utc::now()) — not an error
    let dt_result = svc.get_system_date_and_time().await;
    assert!(
        dt_result.is_ok(),
        "get_system_date_and_time default must return Ok"
    );
}

#[test]
fn test_builder_accepts_service_calls() {
    use onvif_server::DeviceService;
    struct StubDev;
    #[async_trait::async_trait]
    impl DeviceService for StubDev {}

    let result = OnvifServer::builder()
        .port(8080)
        .device_service(StubDev)
        .auth("user", "pass")
        .build();
    assert!(result.is_ok(), "build() should return Ok for valid builder");
}

#[test]
fn test_auth_bypass_includes_get_system_date_and_time() {
    let builder = OnvifServer::builder();
    assert!(
        builder.auth_bypass_set().contains("GetSystemDateAndTime"),
        "GetSystemDateAndTime must be in auth_bypass by default"
    );
}
