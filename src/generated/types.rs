// Auto-generated ONVIF type stubs — Phase 1 skeleton
// Full XSD-derived codegen is a Phase 2+ concern.
// These stubs satisfy INFRA-04 so Phase 2 can build against concrete types immediately.

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}
