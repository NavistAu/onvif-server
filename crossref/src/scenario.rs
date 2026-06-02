//! Declarative ONVIF scenario model (spec §5a). The orchestrator routes on these EXPLICIT
//! fields — never on the scenario name.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    None,
    Usernametoken,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceMode {
    None,
    SrvdProjection,
    SrvdExact,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    #[default]
    Http,
    UdpDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FaultExpectation {
    pub code: String,
    #[serde(default)]
    pub subcode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Capture {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Inject {
    pub name: String,
    pub into: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Step {
    pub operation: String,
    pub schema_id: String,
    pub request_file: String,
    pub expected_status: u16,
    pub outcome: Outcome,
    #[serde(default)]
    pub capture: Vec<Capture>,
    #[serde(default)]
    pub inject: Vec<Inject>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub masks: Vec<String>,
    #[serde(default)]
    pub fault: Option<FaultExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub service: String,
    #[serde(default)]
    pub transport: Transport,
    #[serde(rename = "auth_mode", default)]
    pub auth_mode_opt: Option<AuthMode>,
    #[serde(default)]
    pub reference_mode: Option<ReferenceMode>,
    // single-request form (None when [[steps]] present):
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub schema_id: Option<String>,
    #[serde(default)]
    pub http_method: Option<String>,
    #[serde(default)]
    pub expected_status: Option<u16>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub request_file: Option<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub masks: Vec<String>,
    #[serde(default)]
    pub fault: Option<FaultExpectation>,
    // multi-step form:
    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Scenario {
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    pub fn auth_mode(&self) -> AuthMode {
        self.auth_mode_opt.clone().unwrap_or(AuthMode::None)
    }

    pub fn is_discovery(&self) -> bool {
        self.service == "discovery"
    }

    pub fn is_multistep(&self) -> bool {
        !self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_success() {
        let s = Scenario::from_toml_str(
            r#"
            name = "device_get_device_information_authed"
            service = "device"
            operation = "GetDeviceInformation"
            schema_id = "device-body"
            http_method = "POST"
            expected_status = 200
            outcome = "success"
            auth_mode = "usernametoken"
            reference_mode = "srvd_exact"
            invariants = []
            masks = ["wsa_message_id"]
            request_file = "device_get_device_information_authed.request.xml"
        "#,
        )
        .unwrap();
        assert_eq!(s.service, "device");
        assert_eq!(s.operation.as_deref(), Some("GetDeviceInformation"));
        assert_eq!(s.auth_mode(), AuthMode::Usernametoken);
        assert_eq!(s.reference_mode, Some(ReferenceMode::SrvdExact));
        assert_eq!(s.outcome, Some(Outcome::Success));
        assert_eq!(s.transport, Transport::Http);
        assert!(!s.is_multistep());
        assert!(!s.is_discovery());
        assert_eq!(s.masks, vec!["wsa_message_id".to_string()]);
    }

    #[test]
    fn parses_fault_with_fault_block() {
        let s = Scenario::from_toml_str(
            r#"
            name = "device_auth_missing"
            service = "device"
            operation = "GetDeviceInformation"
            schema_id = "device-body"
            http_method = "POST"
            expected_status = 400
            outcome = "fault"
            auth_mode = "none"
            [fault]
            code = "Sender"
            subcode = "ter:NotAuthorized"
        "#,
        )
        .unwrap();
        assert_eq!(s.outcome, Some(Outcome::Fault));
        let f = s.fault.clone().unwrap();
        assert_eq!(f.code, "Sender");
        assert_eq!(f.subcode.as_deref(), Some("ter:NotAuthorized"));
        assert_eq!(s.auth_mode(), AuthMode::None);
    }

    #[test]
    fn parses_discovery() {
        let s = Scenario::from_toml_str(
            r#"
            name = "discovery_probe_match"
            service = "discovery"
            transport = "udp_discovery"
            schema_id = "none"
            http_method = "none"
            invariants = ["wsa_relates_to_matches_probe", "stable_endpoint_uuid"]
            request_file = "discovery_probe.request.xml"
        "#,
        )
        .unwrap();
        assert!(s.is_discovery());
        assert_eq!(s.transport, Transport::UdpDiscovery);
        assert_eq!(s.invariants.len(), 2);
    }

    #[test]
    fn parses_multistep_events() {
        let s = Scenario::from_toml_str(r#"
            name = "events_pull_messages"
            service = "events"
            auth_mode = "usernametoken"
            reference_mode = "none"
            [[steps]]
            operation = "CreatePullPointSubscription"
            schema_id = "events-body"
            request_file = "events_create_pullpoint.request.xml"
            expected_status = 200
            outcome = "success"
            capture = [{ name = "subId", path = "Envelope/Body/CreatePullPointSubscriptionResponse/SubscriptionReference/Address" }]
            [[steps]]
            operation = "PullMessages"
            schema_id = "events-body"
            request_file = "events_pull_messages.request.xml"
            expected_status = 200
            outcome = "success"
            inject = [{ name = "subId", into = "header:To" }]
            invariants = ["wsa_subscription_id_present"]
        "#).unwrap();
        assert!(s.is_multistep());
        assert_eq!(s.steps.len(), 2);
        assert_eq!(s.steps[0].capture[0].name, "subId");
        assert_eq!(s.steps[1].inject[0].into, "header:To");
        assert_eq!(
            s.steps[1].invariants,
            vec!["wsa_subscription_id_present".to_string()]
        );
    }
}
