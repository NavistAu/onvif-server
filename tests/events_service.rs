use bytes::Bytes;
use onvif_server::service::events::EventServiceHandler;
use onvif_server::EventService;
use soap_server::SoapHandler;
use std::sync::Arc;

struct TestEvents;

#[async_trait::async_trait]
impl EventService for TestEvents {}

fn make_handler() -> EventServiceHandler {
    EventServiceHandler::new(
        Arc::new(TestEvents),
        "http://0.0.0.0:8080/onvif/events_service",
    )
}

#[tokio::test]
async fn events_get_event_properties_response_element() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tev:GetEventProperties xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("GetEventProperties failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tev:GetEventPropertiesResponse"),
        "response must contain tev:GetEventPropertiesResponse: {xml}"
    );
    assert!(
        xml.contains("TopicNamespaceLocation"),
        "response must contain TopicNamespaceLocation: {xml}"
    );
}

#[tokio::test]
async fn events_create_pull_point_subscription_returns_reference() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("CreatePullPointSubscription failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tev:SubscriptionReference"),
        "response must contain tev:SubscriptionReference: {xml}"
    );
    assert!(
        xml.contains("wsa5:Address"),
        "response must contain wsa5:Address: {xml}"
    );
}

#[tokio::test]
async fn events_create_pull_point_subscription_contains_times() {
    let handler = make_handler();
    let body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let result = handler
        .handle(body)
        .await
        .expect("CreatePullPointSubscription failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("wsnt:CurrentTime"),
        "response must contain wsnt:CurrentTime: {xml}"
    );
    assert!(
        xml.contains("wsnt:TerminationTime"),
        "response must contain wsnt:TerminationTime: {xml}"
    );
}

#[tokio::test]
async fn events_pull_messages_returns_response_with_times() {
    let handler = make_handler();
    // First create a subscription to get an ID
    let create_body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let create_result = handler
        .handle(create_body)
        .await
        .expect("CreatePullPointSubscription failed");
    let create_xml = String::from_utf8(create_result.to_vec()).unwrap();

    // Extract SubscriptionId from between tags
    let sub_id = extract_between(&create_xml, "<tev:SubscriptionId>", "</tev:SubscriptionId>")
        .expect("SubscriptionId not found in create response");

    let pull_body = Bytes::from(format!(
        r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></tev:PullMessages>"#,
    ));
    let result = handler
        .handle(pull_body)
        .await
        .expect("PullMessages failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tev:PullMessagesResponse"),
        "response must contain tev:PullMessagesResponse: {xml}"
    );
    assert!(
        xml.contains("tev:CurrentTime"),
        "response must contain tev:CurrentTime: {xml}"
    );
    assert!(
        xml.contains("tev:TerminationTime"),
        "response must contain tev:TerminationTime: {xml}"
    );
    // No NotificationMessage elements
    assert!(
        !xml.contains("NotificationMessage"),
        "response must have no NotificationMessage elements: {xml}"
    );
}

#[tokio::test]
async fn events_unsubscribe_known_subscription_returns_empty_response() {
    let handler = make_handler();
    let create_body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let create_result = handler
        .handle(create_body)
        .await
        .expect("CreatePullPointSubscription failed");
    let create_xml = String::from_utf8(create_result.to_vec()).unwrap();
    let sub_id = extract_between(&create_xml, "<tev:SubscriptionId>", "</tev:SubscriptionId>")
        .expect("SubscriptionId not found");

    let unsub_body = Bytes::from(format!(
        r#"<tev:Unsubscribe xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></tev:Unsubscribe>"#,
    ));
    let result = handler
        .handle(unsub_body)
        .await
        .expect("Unsubscribe failed");
    let xml = String::from_utf8(result.to_vec()).unwrap();
    assert!(
        xml.contains("tev:UnsubscribeResponse"),
        "response must contain tev:UnsubscribeResponse: {xml}"
    );
}

#[tokio::test]
async fn events_pull_messages_unknown_subscription_returns_fault() {
    let handler = make_handler();
    let pull_body = Bytes::from(
        r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>nonexistent-id</tev:SubscriptionId></tev:PullMessages>"#,
    );
    let result = handler.handle(pull_body).await;
    assert!(
        result.is_err(),
        "unknown subscription must return SoapFault"
    );
}

#[tokio::test]
async fn events_unsubscribe_removes_subscription_from_map() {
    let handler = make_handler();
    let create_body = Bytes::from(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#,
    );
    let create_result = handler
        .handle(create_body)
        .await
        .expect("CreatePullPointSubscription failed");
    let create_xml = String::from_utf8(create_result.to_vec()).unwrap();
    let sub_id = extract_between(&create_xml, "<tev:SubscriptionId>", "</tev:SubscriptionId>")
        .expect("SubscriptionId not found");

    // Unsubscribe
    let unsub_body = Bytes::from(format!(
        r#"<tev:Unsubscribe xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></tev:Unsubscribe>"#,
    ));
    handler
        .handle(unsub_body)
        .await
        .expect("Unsubscribe failed");

    // Now PullMessages should fail
    let pull_body = Bytes::from(format!(
        r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><tev:SubscriptionId>{sub_id}</tev:SubscriptionId></tev:PullMessages>"#,
    ));
    let result = handler.handle(pull_body).await;
    assert!(
        result.is_err(),
        "pull after unsubscribe must return SoapFault (subscription removed)"
    );
}

/// Simple string extraction helper — not using regex to keep deps minimal.
fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = s.find(start)? + start.len();
    let end_idx = s[start_idx..].find(end)? + start_idx;
    Some(&s[start_idx..end_idx])
}
