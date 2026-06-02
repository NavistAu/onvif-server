use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use quick_xml::events::Event;
use quick_xml::NsReader;
use soap_server::{escape_text, SoapFault, SoapHandler};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::error::OnvifError;
use crate::service::xml_util::extract_text_ns;
use crate::traits::EventService;

/// ONVIF namespaces accepted for event request elements.
const ONVIF_EVENTS_NS: &[u8] = b"http://www.onvif.org/ver10/events/wsdl";
const ONVIF_SCHEMA_NS: &[u8] = b"http://www.onvif.org/ver10/schema";
/// WS-BaseNotification namespace (used for InitialTerminationTime in CreatePullPointSubscription).
const WSNT_NS: &[u8] = b"http://docs.oasis-open.org/wsn/b-2";

/// Default subscription lifetime in seconds when no (or unparseable)
/// `InitialTerminationTime` is provided by the client.
const DEFAULT_SUBSCRIPTION_SECS: i64 = 60;

struct SubscriptionInfo {
    termination_time: DateTime<Utc>,
}

#[allow(dead_code)]
pub struct EventServiceHandler {
    pub(crate) svc: Arc<dyn EventService>,
    pub(crate) xaddr: String,
    subscriptions: Arc<Mutex<HashMap<String, SubscriptionInfo>>>,
}

impl EventServiceHandler {
    pub fn new(svc: Arc<dyn EventService>, xaddr: impl Into<String>) -> Self {
        Self {
            svc,
            xaddr: xaddr.into(),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SoapHandler for EventServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        // Delegate through handle_with_headers with an empty header slice.
        self.handle_with_headers(body, &[]).await
    }

    /// Override to receive SOAP header fragments so WS-Addressing reference
    /// parameters (echoed SubscriptionId) can be used for routing.
    async fn handle_with_headers(
        &self,
        body: Bytes,
        headers: &[Bytes],
    ) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetEventProperties" => self.handle_get_event_properties().await,
            "CreatePullPointSubscription" => {
                self.handle_create_pull_point_subscription(&body).await
            }
            "PullMessages" => self.handle_pull_messages(&body, headers).await,
            "Unsubscribe" => self.handle_unsubscribe(&body, headers).await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}

fn extract_local_name(body: &Bytes) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader
            .read_resolved_event()
            .map_err(|e| SoapFault::sender(format!("{e}")))?
        {
            (_, Event::Start(e)) | (_, Event::Empty(e)) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?
                    .to_string();
                return Ok(local);
            }
            (_, Event::Eof) => return Err(SoapFault::sender("Empty body".to_string())),
            _ => {}
        }
    }
}

fn extract_text_element(body: &Bytes, element_name: &str) -> Result<String, SoapFault> {
    extract_text_ns(body, element_name, &[ONVIF_EVENTS_NS, ONVIF_SCHEMA_NS])
}

/// Resolve the SubscriptionId for PullMessages / Unsubscribe.
///
/// WS-Addressing reference parameters: the SubscriptionId the server placed inside
/// `ReferenceParameters` at CreatePullPointSubscription time is echoed back by
/// conforming clients as a direct child of `<SOAP:Header>`.  Parse it from the
/// first header element whose local name is `SubscriptionId` (accepted in the
/// ONVIF events namespace or unbound).  Fall back to the body element for
/// backwards-compat with clients that embed it in the operation body.
fn resolve_subscription_id(body: &Bytes, headers: &[Bytes]) -> Result<String, SoapFault> {
    // 1. Try headers first (WS-Addressing reference parameter echo).
    for header in headers {
        if let Ok(id) = extract_text_ns(
            header,
            "SubscriptionId",
            &[ONVIF_EVENTS_NS, ONVIF_SCHEMA_NS],
        ) {
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }

    // 2. Fall back to body element (backward compat).
    extract_text_element(body, "SubscriptionId")
}

/// Parse an `InitialTerminationTime` value into a `Duration`.
///
/// Accepts:
/// - ISO 8601 duration strings: `PT<seconds>S` (e.g. `PT60S`, `PT300S`).
///   Only seconds-level resolution is required for ONVIF pull-point subscriptions.
/// - Absolute RFC 3339 / ISO 8601 datetimes: interpreted as an absolute expiry.
/// - Anything unparseable: returns `None` (caller should use the default).
fn parse_termination_time(value: &str, now: DateTime<Utc>) -> Option<Duration> {
    let trimmed = value.trim();

    // ISO 8601 duration: must start with 'P'.
    if trimmed.starts_with('P') {
        return parse_iso8601_duration(trimmed);
    }

    // Try absolute datetime (RFC 3339).
    if let Ok(absolute) = trimmed.parse::<DateTime<Utc>>() {
        let delta = absolute.signed_duration_since(now);
        if delta > Duration::zero() {
            return Some(delta);
        }
        // Already-past time — use default.
        return None;
    }

    None
}

/// Parse a bounded subset of ISO 8601 durations: `P[nY][nM][nDT[nH][nM][nS]]`.
/// Only handles whole-unit fields; fractional seconds are truncated.
/// Returns `None` if the string does not conform.
fn parse_iso8601_duration(s: &str) -> Option<Duration> {
    // Strip leading 'P'.
    let s = s.strip_prefix('P')?;

    let mut total_secs: i64 = 0;
    let (date_part, time_part) = if let Some(t) = s.split_once('T') {
        (t.0, Some(t.1))
    } else {
        (s, None)
    };

    // Parse date fields: nY, nM, nD.
    let mut remaining = date_part;
    total_secs += consume_duration_field(&mut remaining, 'Y')? * 365 * 86400;
    total_secs += consume_duration_field(&mut remaining, 'M')? * 30 * 86400;
    total_secs += consume_duration_field(&mut remaining, 'D')? * 86400;

    // Parse time fields: nH, nM, nS.
    if let Some(time) = time_part {
        let mut rem = time;
        total_secs += consume_duration_field(&mut rem, 'H')? * 3600;
        total_secs += consume_duration_field(&mut rem, 'M')? * 60;
        // Seconds: allow fractional (truncate).
        total_secs += consume_duration_seconds(&mut rem)?;
        if !rem.is_empty() {
            return None;
        }
    } else if !remaining.is_empty() {
        return None;
    }

    Some(Duration::seconds(total_secs))
}

/// Consume an optional `<integer><designator>` prefix from `*s`, returning the
/// integer (or 0 if this designator is absent).  Returns `None` on parse error.
fn consume_duration_field(s: &mut &str, designator: char) -> Option<i64> {
    if let Some(pos) = s.find(designator) {
        // Everything before the designator must be a non-empty integer.
        let num_str = &s[..pos];
        if num_str.is_empty() {
            return None;
        }
        let n: i64 = num_str.parse().ok()?;
        *s = &s[pos + designator.len_utf8()..];
        Some(n)
    } else {
        Some(0)
    }
}

/// Consume optional `<integer-or-float>S` from the front of `*s`, returning the
/// truncated integer seconds.  Returns 0 (not None) if 'S' is absent.
fn consume_duration_seconds(s: &mut &str) -> Option<i64> {
    if let Some(pos) = s.find('S') {
        let num_str = &s[..pos];
        if num_str.is_empty() {
            return None;
        }
        // Accept integer or decimal (e.g. "30.5S" → 30).
        let n: i64 = if let Some(dot) = num_str.find('.') {
            num_str[..dot].parse().ok()?
        } else {
            num_str.parse().ok()?
        };
        *s = &s[pos + 1..];
        Some(n)
    } else {
        Some(0)
    }
}

impl EventServiceHandler {
    async fn handle_get_event_properties(&self) -> Result<Bytes, SoapFault> {
        let xml = r#"<tev:GetEventPropertiesResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2" xmlns:wstop="http://docs.oasis-open.org/wsn/t-1"><tev:TopicNamespaceLocation>http://www.onvif.org/onvif/ver10/topics/topicns.xml</tev:TopicNamespaceLocation><wsnt:FixedTopicSet>true</wsnt:FixedTopicSet><wstop:TopicSet/><wsnt:TopicExpressionDialect>http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete</wsnt:TopicExpressionDialect><wsnt:TopicExpressionDialect>http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet</wsnt:TopicExpressionDialect><tev:MessageContentFilterDialect>http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter</tev:MessageContentFilterDialect><tev:MessageContentSchemaLocation>http://www.onvif.org/ver10/schema/onvif.xsd</tev:MessageContentSchemaLocation></tev:GetEventPropertiesResponse>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_create_pull_point_subscription(
        &self,
        body: &Bytes,
    ) -> Result<Bytes, SoapFault> {
        let sub_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Parse InitialTerminationTime from the request body.
        // The element may appear in the ONVIF events namespace, the WS-Notification
        // namespace (wsnt), or unbound — accept all three.
        // Accept either an ISO 8601 duration (e.g. PT60S) or an absolute datetime.
        // Fall back to DEFAULT_SUBSCRIPTION_SECS if absent or unparseable.
        let lifetime = extract_text_ns(
            body,
            "InitialTerminationTime",
            &[ONVIF_EVENTS_NS, ONVIF_SCHEMA_NS, WSNT_NS],
        )
        .ok()
        .and_then(|v| parse_termination_time(&v, now))
        .unwrap_or_else(|| Duration::seconds(DEFAULT_SUBSCRIPTION_SECS));

        let termination = now + lifetime;

        {
            let mut subs = self
                .subscriptions
                .lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            subs.insert(
                sub_id.clone(),
                SubscriptionInfo {
                    termination_time: termination,
                },
            );
        }

        let xml = format!(
            r#"<tev:CreatePullPointSubscriptionResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsa5="http://www.w3.org/2005/08/addressing" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:SubscriptionReference><wsa5:Address>{xaddr}</wsa5:Address><wsa5:ReferenceParameters><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></wsa5:ReferenceParameters></tev:SubscriptionReference><wsnt:CurrentTime>{current}</wsnt:CurrentTime><wsnt:TerminationTime>{termination}</wsnt:TerminationTime></tev:CreatePullPointSubscriptionResponse>"#,
            xaddr = escape_text(&self.xaddr),
            sub_id = escape_text(&sub_id),
            current = now.to_rfc3339(),
            termination = termination.to_rfc3339(),
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_pull_messages(
        &self,
        body: &Bytes,
        headers: &[Bytes],
    ) -> Result<Bytes, SoapFault> {
        // Resolve SubscriptionId: header (WS-Addressing ref param) first, body fallback.
        let sub_id = resolve_subscription_id(body, headers)?;

        let now = Utc::now();
        let termination_time = {
            let subs = self
                .subscriptions
                .lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            match subs.get(&sub_id) {
                Some(info) if info.termination_time > now => info.termination_time,
                Some(_) => {
                    // Subscription exists but has expired.
                    // Log detail server-side; return generic fault reason to client (#12).
                    eprintln!("[onvif-events] PullMessages: subscription expired (id not echoed to client)");
                    return Err(SoapFault::sender("Subscription not found".to_string()));
                }
                None => {
                    // Log detail server-side; return generic fault reason to client (#12).
                    eprintln!("[onvif-events] PullMessages: unknown subscription (id not echoed to client)");
                    return Err(SoapFault::sender("Subscription not found".to_string()));
                }
            }
        };

        let xml = format!(
            r#"<tev:PullMessagesResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:CurrentTime>{current}</tev:CurrentTime><tev:TerminationTime>{termination}</tev:TerminationTime></tev:PullMessagesResponse>"#,
            current = now.to_rfc3339(),
            termination = termination_time.to_rfc3339(),
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_unsubscribe(
        &self,
        body: &Bytes,
        headers: &[Bytes],
    ) -> Result<Bytes, SoapFault> {
        // Resolve SubscriptionId: header (WS-Addressing ref param) first, body fallback.
        let sub_id = resolve_subscription_id(body, headers)?;

        {
            let mut subs = self
                .subscriptions
                .lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            if subs.remove(&sub_id).is_none() {
                // Unknown or already-expired subscription — log server-side, fault to client (#12).
                eprintln!("[onvif-events] Unsubscribe: unknown or expired subscription (id not echoed to client)");
                return Err(SoapFault::sender("Subscription not found".to_string()));
            }
        }

        let xml =
            r#"<tev:UnsubscribeResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_iso8601_duration unit tests ─────────────────────────────────────

    #[test]
    fn parse_pt60s_returns_60_seconds() {
        let d = parse_iso8601_duration("PT60S").unwrap();
        assert_eq!(d.num_seconds(), 60);
    }

    #[test]
    fn parse_pt300s_returns_300_seconds() {
        let d = parse_iso8601_duration("PT300S").unwrap();
        assert_eq!(d.num_seconds(), 300);
    }

    #[test]
    fn parse_pt1h_returns_3600_seconds() {
        let d = parse_iso8601_duration("PT1H").unwrap();
        assert_eq!(d.num_seconds(), 3600);
    }

    #[test]
    fn parse_pt1h30m_returns_5400_seconds() {
        let d = parse_iso8601_duration("PT1H30M").unwrap();
        assert_eq!(d.num_seconds(), 5400);
    }

    #[test]
    fn parse_p1d_returns_86400_seconds() {
        let d = parse_iso8601_duration("P1D").unwrap();
        assert_eq!(d.num_seconds(), 86400);
    }

    #[test]
    fn parse_empty_string_returns_none() {
        assert!(parse_iso8601_duration("").is_none());
    }

    #[test]
    fn parse_garbage_returns_none() {
        assert!(parse_iso8601_duration("garbage").is_none());
        assert!(parse_iso8601_duration("60S").is_none()); // missing 'P'
    }

    // ── parse_termination_time absolute datetime tests ────────────────────────

    #[test]
    fn parse_future_absolute_datetime_returns_positive_duration() {
        let now = Utc::now();
        let future = now + Duration::seconds(120);
        let s = future.to_rfc3339();
        let d = parse_termination_time(&s, now).unwrap();
        // Should be approximately 120 seconds (allow ±2s for rounding).
        assert!(d.num_seconds() >= 118 && d.num_seconds() <= 122);
    }

    #[test]
    fn parse_past_absolute_datetime_returns_none() {
        let now = Utc::now();
        let past = now - Duration::seconds(10);
        let s = past.to_rfc3339();
        assert!(parse_termination_time(&s, now).is_none());
    }

    // ── resolve_subscription_id: header-first, body-fallback ─────────────────

    #[test]
    fn resolve_sub_id_from_header_takes_priority_over_body() {
        let body = Bytes::from(
            r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>body-id</tev:SubscriptionId></tev:PullMessages>"#,
        );
        let header = Bytes::from(
            r#"<tev:SubscriptionId xmlns:tev="http://www.onvif.org/ver10/events/wsdl">header-id</tev:SubscriptionId>"#,
        );
        let id = resolve_subscription_id(&body, &[header]).unwrap();
        assert_eq!(id, "header-id");
    }

    #[test]
    fn resolve_sub_id_falls_back_to_body_when_no_header() {
        let body = Bytes::from(
            r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>body-id</tev:SubscriptionId></tev:PullMessages>"#,
        );
        let id = resolve_subscription_id(&body, &[]).unwrap();
        assert_eq!(id, "body-id");
    }

    #[test]
    fn resolve_sub_id_unbound_namespace_in_header_accepted() {
        // Header element with no namespace binding — must be accepted.
        let body = Bytes::from(
            r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>body-id</tev:SubscriptionId></tev:PullMessages>"#,
        );
        let header = Bytes::from(r#"<SubscriptionId>bare-id</SubscriptionId>"#);
        let id = resolve_subscription_id(&body, &[header]).unwrap();
        assert_eq!(id, "bare-id");
    }
}
