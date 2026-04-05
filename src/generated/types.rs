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

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeDefinition {
    Fixed,
    Configurable,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub scope_def: ScopeDefinition,
    pub scope_item: String,
}

#[derive(Debug, Clone)]
pub struct HostnameInformation {
    pub from_dhcp: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub token: String,
    pub enabled: bool,
    pub name: String,
    pub hw_address: String,
    pub mtu: u32,
}
