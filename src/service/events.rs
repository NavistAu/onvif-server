use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::OnvifError;
use crate::traits::EventService;

struct SubscriptionInfo {
    termination_time: DateTime<Utc>,
}

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
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetEventProperties"           => self.handle_get_event_properties().await,
            "CreatePullPointSubscription"  => self.handle_create_pull_point_subscription().await,
            "PullMessages"                 => self.handle_pull_messages(&body).await,
            "Unsubscribe"                  => self.handle_unsubscribe(&body).await,
            _ => Err(OnvifError::ActionNotSupported.into_soap_fault()),
        }
    }
}

fn extract_local_name(body: &Bytes) -> Result<String, SoapFault> {
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
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
    let mut reader = NsReader::from_reader(body.as_ref());
    reader.config_mut().trim_text(true);
    let mut inside_target = false;
    loop {
        match reader.read_resolved_event().map_err(|e| SoapFault::sender(format!("{e}")))? {
            (_, Event::Start(e)) => {
                let local_name = e.local_name();
                let local = std::str::from_utf8(local_name.as_ref())
                    .map_err(|e| SoapFault::sender(format!("{e}")))?;
                if local == element_name {
                    inside_target = true;
                }
            }
            (_, Event::Text(t)) if inside_target => {
                return std::str::from_utf8(t.as_ref())
                    .map(|s| s.to_owned())
                    .map_err(|e| SoapFault::sender(format!("{e}")));
            }
            (_, Event::Eof) => return Err(SoapFault::sender(
                format!("Element {element_name} not found in body")
            )),
            _ => {}
        }
    }
}

impl EventServiceHandler {
    async fn handle_get_event_properties(&self) -> Result<Bytes, SoapFault> {
        let xml = r#"<tev:GetEventPropertiesResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2" xmlns:wstop="http://docs.oasis-open.org/wsn/t-1"><tev:TopicNamespaceLocation>http://www.onvif.org/onvif/ver10/topics/topicns.xml</tev:TopicNamespaceLocation><wsnt:FixedTopicSet>true</wsnt:FixedTopicSet><wstop:TopicSet/><wsnt:TopicExpressionDialect>http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete</wsnt:TopicExpressionDialect><wsnt:TopicExpressionDialect>http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet</wsnt:TopicExpressionDialect><tev:MessageContentFilterDialect>http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter</tev:MessageContentFilterDialect></tev:GetEventPropertiesResponse>"#;
        Ok(Bytes::from(xml))
    }

    async fn handle_create_pull_point_subscription(&self) -> Result<Bytes, SoapFault> {
        let sub_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let termination = now + chrono::Duration::seconds(60);

        {
            let mut subs = self.subscriptions.lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            subs.insert(sub_id.clone(), SubscriptionInfo { termination_time: termination });
        }

        let xml = format!(
            r#"<tev:CreatePullPointSubscriptionResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsa5="http://www.w3.org/2005/08/addressing" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:SubscriptionReference><wsa5:Address>{xaddr}</wsa5:Address><wsa5:ReferenceParameters><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></wsa5:ReferenceParameters></tev:SubscriptionReference><wsnt:CurrentTime>{current}</wsnt:CurrentTime><wsnt:TerminationTime>{termination}</wsnt:TerminationTime></tev:CreatePullPointSubscriptionResponse>"#,
            xaddr = self.xaddr,
            sub_id = sub_id,
            current = now.to_rfc3339(),
            termination = termination.to_rfc3339(),
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_pull_messages(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let sub_id = extract_text_element(body, "SubscriptionId")?;

        let termination_time = {
            let subs = self.subscriptions.lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            match subs.get(&sub_id) {
                Some(info) => info.termination_time,
                None => return Err(SoapFault::sender(format!("Unknown subscription: {sub_id}"))),
            }
        };

        let now = Utc::now();
        let xml = format!(
            r#"<tev:PullMessagesResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2"><tev:CurrentTime>{current}</tev:CurrentTime><tev:TerminationTime>{termination}</tev:TerminationTime></tev:PullMessagesResponse>"#,
            current = now.to_rfc3339(),
            termination = termination_time.to_rfc3339(),
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_unsubscribe(&self, body: &Bytes) -> Result<Bytes, SoapFault> {
        let sub_id = extract_text_element(body, "SubscriptionId")?;

        {
            let mut subs = self.subscriptions.lock()
                .map_err(|e| SoapFault::sender(format!("lock poisoned: {e}")))?;
            subs.remove(&sub_id);
        }

        let xml = r#"<tev:UnsubscribeResponse xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#;
        Ok(Bytes::from(xml))
    }
}
