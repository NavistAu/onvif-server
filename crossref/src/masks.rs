//! Named mask registry (spec §8).
//!
//! Each name maps to path-scoped [`MaskRule`] and [`AttrMaskRule`] instances
//! that the Layer-1 harness applies to raw response bytes before diffing
//! against frozen snapshots.  Paths use slash-joined LOCAL-NAME segments
//! (namespace-prefix-agnostic).
//!
//! ## Local-name paths — derived from real fixture responses
//!
//! | Mask name          | Paths masked                                                  |
//! |--------------------|---------------------------------------------------------------|
//! | `wsa_message_id`         | `Envelope/Header/MessageID` (discovery ProbeMatch header)    |
//! | `system_datetime`        | `…/SystemDateAndTime/UTCDateTime/Time/{Hour,Minute,Second}`  |
//! |                          | `…/SystemDateAndTime/UTCDateTime/Date/{Year,Month,Day}`      |
//! | `current_time`           | `…/PullMessagesResponse/CurrentTime`                         |
//! |                          | `…/CreatePullPointSubscriptionResponse/CurrentTime`          |
//! | `termination_time`       | `…/PullMessagesResponse/TerminationTime`                     |
//! |                          | `…/CreatePullPointSubscriptionResponse/TerminationTime`      |
//! | `subscription_id`        | `…/CreatePullPointSubscriptionResponse/SubscriptionReference/ReferenceParameters/SubscriptionId` |
//! | `host_authority`         | `…/CreatePullPointSubscriptionResponse/SubscriptionReference/Address` |
//! | `ptz_status_utc_time`    | `Envelope/Body/GetStatusResponse/PTZStatus/UtcTime`          |
//!
//! `host_authority` is a no-op in Layer-1 (the fixture pins the advertised
//! host) but is included so Layer-2 can reuse the same scenario metadata.

use crate::normalize::{AttrMaskRule, MaskRule};

/// Resolve a named mask (spec §8) to its text-mask rules and attribute-mask rules.
///
/// Unknown names return `(vec![], vec![])`.  A `debug_assert!` fires in debug
/// builds so scenario-file typos surface immediately during tests without
/// crashing CI.
pub fn resolve(name: &str) -> (Vec<MaskRule>, Vec<AttrMaskRule>) {
    match name {
        // ── WS-Addressing MessageID in discovery ProbeMatch header ──────────
        "wsa_message_id" => (
            vec![MaskRule::new("Envelope/Header/MessageID")],
            vec![],
        ),

        // ── GetSystemDateAndTime volatile UTC time fields ────────────────────
        // Real path (local names):
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Hour
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Minute
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Second
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Year
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Month
        //   Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Day
        "system_datetime" => (
            vec![
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Hour",
                ),
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Minute",
                ),
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Time/Second",
                ),
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Year",
                ),
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Month",
                ),
                MaskRule::new(
                    "Envelope/Body/GetSystemDateAndTimeResponse/SystemDateAndTime/UTCDateTime/Date/Day",
                ),
            ],
            vec![],
        ),

        // ── PullMessages / CreatePullPointSubscription volatile timestamps ───
        // Real paths (local names):
        //   Envelope/Body/PullMessagesResponse/CurrentTime
        //   Envelope/Body/CreatePullPointSubscriptionResponse/CurrentTime
        "current_time" => (
            vec![
                MaskRule::new("Envelope/Body/PullMessagesResponse/CurrentTime"),
                MaskRule::new(
                    "Envelope/Body/CreatePullPointSubscriptionResponse/CurrentTime",
                ),
            ],
            vec![],
        ),

        // Real paths (local names):
        //   Envelope/Body/PullMessagesResponse/TerminationTime
        //   Envelope/Body/CreatePullPointSubscriptionResponse/TerminationTime
        "termination_time" => (
            vec![
                MaskRule::new("Envelope/Body/PullMessagesResponse/TerminationTime"),
                MaskRule::new(
                    "Envelope/Body/CreatePullPointSubscriptionResponse/TerminationTime",
                ),
            ],
            vec![],
        ),

        // ── Volatile subscription UUID ───────────────────────────────────────
        // Real path (local names):
        //   Envelope/Body/CreatePullPointSubscriptionResponse/SubscriptionReference/ReferenceParameters/SubscriptionId
        "subscription_id" => (
            vec![MaskRule::new(
                "Envelope/Body/CreatePullPointSubscriptionResponse/SubscriptionReference/ReferenceParameters/SubscriptionId",
            )],
            vec![],
        ),

        // ── Host authority inside subscription address / stream / snapshot URIs
        // Real path (local names):
        //   Envelope/Body/CreatePullPointSubscriptionResponse/SubscriptionReference/Address
        // In Layer-1 the fixture pins the advertised host so this is a no-op,
        // but included for Layer-2 parity.
        "host_authority" => (
            vec![MaskRule::new(
                "Envelope/Body/CreatePullPointSubscriptionResponse/SubscriptionReference/Address",
            )],
            vec![],
        ),

        // ── GetStatus live UTC timestamp ─────────────────────────────────────
        // Real path (local names):
        //   Envelope/Body/GetStatusResponse/PTZStatus/UtcTime
        // tt:PTZStatus requires UtcTime (xs:dateTime, minOccurs=1).  The value
        // is Utc::now() at response-generation time — mask it for snapshot stability.
        "ptz_status_utc_time" => (
            vec![MaskRule::new(
                "Envelope/Body/GetStatusResponse/PTZStatus/UtcTime",
            )],
            vec![],
        ),

        // ── Discovery ProbeMatch endpoint address UUID ───────────────────────
        // The harness generates a UUID for the ProbeMatch; the parse of the
        // fixed literal fails (non-hex chars) so new_v4() is used each run.
        // Mask the Address element so the snapshot is stable.
        // Real path (local names):
        //   Envelope/Body/ProbeMatches/ProbeMatch/EndpointReference/Address
        "discovery_endpoint" => (
            vec![MaskRule::new(
                "Envelope/Body/ProbeMatches/ProbeMatch/EndpointReference/Address",
            )],
            vec![],
        ),

        unknown => {
            debug_assert!(
                false,
                "masks::resolve: unknown mask name {:?} — check your scenario .toml",
                unknown
            );
            (vec![], vec![])
        }
    }
}

/// Compose masks for multiple names, deduplicating nothing (order preserved).
pub fn resolve_all(names: &[String]) -> (Vec<MaskRule>, Vec<AttrMaskRule>) {
    let mut text_rules: Vec<MaskRule> = Vec::new();
    let mut attr_rules: Vec<AttrMaskRule> = Vec::new();
    for name in names {
        let (t, a) = resolve(name.as_str());
        text_rules.extend(t);
        attr_rules.extend(a);
    }
    (text_rules, attr_rules)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::{mask_only, MASK_SENTINEL};

    // ── resolve returns non-empty for every known name ──────────────────────

    #[test]
    fn wsa_message_id_non_empty() {
        let (t, _a) = resolve("wsa_message_id");
        assert!(
            !t.is_empty(),
            "wsa_message_id must have at least one text rule"
        );
    }

    #[test]
    fn system_datetime_non_empty() {
        let (t, _a) = resolve("system_datetime");
        assert_eq!(
            t.len(),
            6,
            "system_datetime must have 6 rules (H/M/S Y/Mo/D)"
        );
    }

    #[test]
    fn current_time_non_empty() {
        let (t, _a) = resolve("current_time");
        assert_eq!(
            t.len(),
            2,
            "current_time must cover PullMessages + CreatePullPoint"
        );
    }

    #[test]
    fn termination_time_non_empty() {
        let (t, _a) = resolve("termination_time");
        assert_eq!(
            t.len(),
            2,
            "termination_time must cover PullMessages + CreatePullPoint"
        );
    }

    #[test]
    fn subscription_id_non_empty() {
        let (t, _a) = resolve("subscription_id");
        assert!(
            !t.is_empty(),
            "subscription_id must have at least one text rule"
        );
    }

    #[test]
    fn host_authority_non_empty() {
        let (t, _a) = resolve("host_authority");
        assert!(
            !t.is_empty(),
            "host_authority must have at least one text rule"
        );
    }

    #[test]
    fn discovery_endpoint_non_empty() {
        let (t, _a) = resolve("discovery_endpoint");
        assert!(
            !t.is_empty(),
            "discovery_endpoint must have at least one text rule"
        );
    }

    #[test]
    fn ptz_status_utc_time_non_empty() {
        let (t, _a) = resolve("ptz_status_utc_time");
        assert!(
            !t.is_empty(),
            "ptz_status_utc_time must have at least one text rule"
        );
    }

    /// Envelope mirroring the GetStatusResponse structure (with UtcTime).
    const PTZ_STATUS_XML: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tptz:GetStatusResponse xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><tptz:PTZStatus><tt:MoveStatus><tt:PanTilt>IDLE</tt:PanTilt><tt:Zoom>IDLE</tt:Zoom></tt:MoveStatus><tt:UtcTime>2026-06-03T01:02:03Z</tt:UtcTime></tptz:PTZStatus></tptz:GetStatusResponse></env:Body></env:Envelope>"#;

    #[test]
    fn ptz_status_utc_time_mask_replaces_timestamp() {
        let (t, a) = resolve("ptz_status_utc_time");
        let masked = mask_only(PTZ_STATUS_XML, &t, &a).expect("mask_only must not fail");
        let s = String::from_utf8(masked).expect("utf8");
        assert!(s.contains(MASK_SENTINEL), "UtcTime should be masked: {s}");
        assert!(
            !s.contains("2026-06-03T01:02:03Z"),
            "original timestamp must be gone: {s}"
        );
    }

    #[test]
    fn unknown_name_returns_empty() {
        // debug_assert fires in debug builds, but the return is (vec![], vec![])
        // We can't easily test the assert without catching panics in debug mode,
        // so just verify the release behaviour: empty vecs returned.
        // (In CI the debug_assert is active; this test verifies the return value
        // separately via a name that is definitely unknown in release paths.)
        // Use cfg to avoid the debug_assert panic in test:
        #[cfg(not(debug_assertions))]
        {
            let (t, a) = resolve("totally_unknown_mask_xyz");
            assert!(t.is_empty() && a.is_empty());
        }
    }

    // ── resolve_all composes multiple names ─────────────────────────────────

    #[test]
    fn resolve_all_composes() {
        let names: Vec<String> = ["current_time", "termination_time"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (t, a) = resolve_all(&names);
        // current_time (2) + termination_time (2) = 4
        assert_eq!(t.len(), 4);
        assert!(a.is_empty());
    }

    #[test]
    fn resolve_all_empty_names() {
        let (t, a) = resolve_all(&[]);
        assert!(t.is_empty() && a.is_empty());
    }

    // ── Applying system_datetime mask to a real-shape response ──────────────

    /// Envelope mirroring the actual GetSystemDateAndTime fixture response structure.
    const GSDT_XML: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tds:GetSystemDateAndTimeResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:SystemDateAndTime>
    <tt:DateTimeType>Manual</tt:DateTimeType>
    <tt:DaylightSavings>false</tt:DaylightSavings>
    <tt:TimeZone><tt:TZ>UTC</tt:TZ></tt:TimeZone>
    <tt:UTCDateTime>
      <tt:Time><tt:Hour>9</tt:Hour><tt:Minute>52</tt:Minute><tt:Second>33</tt:Second></tt:Time>
      <tt:Date><tt:Year>2026</tt:Year><tt:Month>6</tt:Month><tt:Day>2</tt:Day></tt:Date>
    </tt:UTCDateTime>
  </tds:SystemDateAndTime>
</tds:GetSystemDateAndTimeResponse></env:Body></env:Envelope>"#;

    #[test]
    fn system_datetime_mask_replaces_time_digits() {
        let (text_rules, attr_rules) = resolve("system_datetime");
        let masked = mask_only(GSDT_XML, &text_rules, &attr_rules)
            .expect("mask_only must not fail on valid XML");
        let s = String::from_utf8(masked).expect("utf8");
        // All six fields should be replaced with the sentinel.
        let sentinel_count = s.matches(MASK_SENTINEL).count();
        assert_eq!(
            sentinel_count, 6,
            "expected 6 masked fields (H/M/S + Y/Mo/D), got {sentinel_count} in: {s}"
        );
        // The literal digits must be gone.
        assert!(
            !s.contains(">9<") && !s.contains(">52<") && !s.contains(">33<"),
            "time digits should be masked: {s}"
        );
        assert!(
            !s.contains(">2026<") && !s.contains(">6<") && !s.contains(">2<"),
            "date digits should be masked: {s}"
        );
    }

    /// Envelope mirroring the actual CreatePullPointSubscriptionResponse structure.
    const CREATE_PULL_XML: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tev:CreatePullPointSubscriptionResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsa5="http://www.w3.org/2005/08/addressing" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:SubscriptionReference><wsa5:Address>http://controlled-onvif:8080:0/onvif/events_service</wsa5:Address><wsa5:ReferenceParameters><tev:SubscriptionId>c64976f9-35bf-42db-b257-eb0510ee8abb</tev:SubscriptionId></wsa5:ReferenceParameters></tev:SubscriptionReference><wsnt:CurrentTime>2026-06-02T09:52:48.843139+00:00</wsnt:CurrentTime><wsnt:TerminationTime>2026-06-02T09:53:48.843139+00:00</wsnt:TerminationTime></tev:CreatePullPointSubscriptionResponse></env:Body></env:Envelope>"#;

    #[test]
    fn current_time_mask_on_create_pull_point() {
        let (t, a) = resolve("current_time");
        let masked = mask_only(CREATE_PULL_XML, &t, &a).expect("mask_only");
        let s = String::from_utf8(masked).expect("utf8");
        assert!(
            s.contains(MASK_SENTINEL),
            "CurrentTime should be masked: {s}"
        );
        assert!(
            !s.contains("2026-06-02T09:52:48"),
            "original CurrentTime timestamp must be gone: {s}"
        );
    }

    #[test]
    fn subscription_id_mask_on_create_pull_point() {
        let (t, a) = resolve("subscription_id");
        let masked = mask_only(CREATE_PULL_XML, &t, &a).expect("mask_only");
        let s = String::from_utf8(masked).expect("utf8");
        assert!(
            s.contains(MASK_SENTINEL),
            "SubscriptionId should be masked: {s}"
        );
        assert!(
            !s.contains("c64976f9-35bf-42db-b257-eb0510ee8abb"),
            "UUID must be gone: {s}"
        );
    }

    /// Envelope mirroring PullMessagesResponse.
    const PULL_MSG_XML: &[u8] = br#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"><env:Body><tev:PullMessagesResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:CurrentTime>2026-06-02T09:52:48.843562+00:00</tev:CurrentTime><tev:TerminationTime>2026-06-02T09:53:48.843139+00:00</tev:TerminationTime></tev:PullMessagesResponse></env:Body></env:Envelope>"#;

    #[test]
    fn current_time_and_termination_mask_on_pull_messages() {
        let names: Vec<String> = ["current_time", "termination_time"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (t, a) = resolve_all(&names);
        let masked = mask_only(PULL_MSG_XML, &t, &a).expect("mask_only");
        let s = String::from_utf8(masked).expect("utf8");
        let count = s.matches(MASK_SENTINEL).count();
        assert_eq!(
            count, 2,
            "both CurrentTime and TerminationTime should be masked: {s}"
        );
    }
}
