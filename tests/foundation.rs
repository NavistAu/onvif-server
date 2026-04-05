use onvif_server::{
    OnvifError,
    PROFILE_TOKEN,
    VIDEO_SOURCE_TOKEN,
    PTZ_NODE_TOKEN,
    PTZ_CONFIG_TOKEN,
    TRANSLATION_SPACE_FOV,
};

#[test]
fn test_not_implemented_fault_has_ter_namespace() {
    let fault = OnvifError::NotImplemented.into_soap_fault();
    let detail = fault.detail.expect("detail must be present for ONVIF faults");
    assert!(
        detail.contains(r#"xmlns:ter="http://www.onvif.org/ver10/error""#),
        "detail must contain ONVIF ter namespace declaration, got: {detail}"
    );
}

#[test]
fn test_token_constants_defined() {
    assert!(!PROFILE_TOKEN.is_empty(), "PROFILE_TOKEN must not be empty");
    assert!(!VIDEO_SOURCE_TOKEN.is_empty(), "VIDEO_SOURCE_TOKEN must not be empty");
    assert!(!PTZ_NODE_TOKEN.is_empty(), "PTZ_NODE_TOKEN must not be empty");
    assert!(!PTZ_CONFIG_TOKEN.is_empty(), "PTZ_CONFIG_TOKEN must not be empty");
    assert!(!TRANSLATION_SPACE_FOV.is_empty(), "TRANSLATION_SPACE_FOV must not be empty");
    assert!(
        TRANSLATION_SPACE_FOV.contains("TranslationSpaceFov"),
        "TRANSLATION_SPACE_FOV must contain 'TranslationSpaceFov', got: {TRANSLATION_SPACE_FOV}"
    );
}

// Stubs for future plans — implemented in later plans

#[test]
#[ignore = "implemented in later plans"]
fn test_embedded_wsdl_loader() {}

#[test]
#[ignore = "implemented in later plans"]
fn test_not_implemented_returns_error() {}

#[test]
#[ignore = "implemented in later plans"]
fn test_builder_accepts_service_calls() {}

#[test]
#[ignore = "implemented in later plans"]
fn test_auth_bypass_includes_get_system_date_and_time() {}
