use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{Datelike, Timelike};
use soap_server::{SoapHandler, SoapFault};
use quick_xml::NsReader;
use quick_xml::events::Event;

use crate::error::OnvifError;
use crate::traits::DeviceService;

pub struct DeviceServiceHandler {
    pub(crate) svc: Arc<dyn DeviceService>,
    pub(crate) xaddr: String, // e.g. "http://192.168.1.10:8080/onvif/device_service"
}

impl DeviceServiceHandler {
    pub fn new(svc: Arc<dyn DeviceService>, xaddr: impl Into<String>) -> Self {
        Self {
            svc,
            xaddr: xaddr.into(),
        }
    }
}

#[async_trait]
impl SoapHandler for DeviceServiceHandler {
    async fn handle(&self, body: Bytes) -> Result<Bytes, SoapFault> {
        let op = extract_local_name(&body)?;
        match op.as_str() {
            "GetSystemDateAndTime" => self.handle_get_system_date_and_time().await,
            "GetCapabilities"      => self.handle_get_capabilities().await,
            "GetServices"          => self.handle_get_services().await,
            "GetDeviceInformation" => self.handle_get_device_information().await,
            "GetScopes"            => self.handle_get_scopes().await,
            "GetHostname"          => self.handle_get_hostname().await,
            "GetNetworkInterfaces" => self.handle_get_network_interfaces().await,
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

impl DeviceServiceHandler {
    async fn handle_get_system_date_and_time(&self) -> Result<Bytes, SoapFault> {
        let dt = self.svc.get_system_date_and_time().await
            .map_err(|e| e.into_soap_fault())?;
        let xml = format!(
            r#"<tds:GetSystemDateAndTimeResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:SystemDateAndTime>
    <tt:DateTimeType>Manual</tt:DateTimeType>
    <tt:DaylightSavings>false</tt:DaylightSavings>
    <tt:TimeZone><tt:TZ>UTC</tt:TZ></tt:TimeZone>
    <tt:UTCDateTime>
      <tt:Time><tt:Hour>{}</tt:Hour><tt:Minute>{}</tt:Minute><tt:Second>{}</tt:Second></tt:Time>
      <tt:Date><tt:Year>{}</tt:Year><tt:Month>{}</tt:Month><tt:Day>{}</tt:Day></tt:Date>
    </tt:UTCDateTime>
  </tds:SystemDateAndTime>
</tds:GetSystemDateAndTimeResponse>"#,
            dt.hour(), dt.minute(), dt.second(),
            dt.year(), dt.month(), dt.day()
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_capabilities(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<tds:GetCapabilitiesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:Capabilities>
    <tt:Device>
      <tt:XAddr>{}</tt:XAddr>
    </tt:Device>
  </tds:Capabilities>
</tds:GetCapabilitiesResponse>"#,
            self.xaddr
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_services(&self) -> Result<Bytes, SoapFault> {
        let xml = format!(
            r#"<tds:GetServicesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <tds:Service>
    <tds:Namespace>http://www.onvif.org/ver10/device/wsdl</tds:Namespace>
    <tds:XAddr>{}</tds:XAddr>
    <tds:Version><tt:Major>2</tt:Major><tt:Minor>42</tt:Minor></tds:Version>
  </tds:Service>
</tds:GetServicesResponse>"#,
            self.xaddr
        );
        Ok(Bytes::from(xml))
    }

    async fn handle_get_device_information(&self) -> Result<Bytes, SoapFault> {
        let info = self.svc.get_device_information().await
            .map_err(|e| e.into_soap_fault())?;
        let xml = format!(
            r#"<tds:GetDeviceInformationResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <tds:Manufacturer>{}</tds:Manufacturer>
  <tds:Model>{}</tds:Model>
  <tds:FirmwareVersion>{}</tds:FirmwareVersion>
  <tds:SerialNumber>{}</tds:SerialNumber>
  <tds:HardwareId>{}</tds:HardwareId>
</tds:GetDeviceInformationResponse>"#,
            info.manufacturer, info.model, info.firmware_version,
            info.serial_number, info.hardware_id
        );
        Ok(Bytes::from(xml))
    }

    // GetScopes, GetHostname, GetNetworkInterfaces — implemented in Plan 02
    async fn handle_get_scopes(&self) -> Result<Bytes, SoapFault> {
        Err(OnvifError::ActionNotSupported.into_soap_fault())
    }

    async fn handle_get_hostname(&self) -> Result<Bytes, SoapFault> {
        Err(OnvifError::ActionNotSupported.into_soap_fault())
    }

    async fn handle_get_network_interfaces(&self) -> Result<Bytes, SoapFault> {
        Err(OnvifError::ActionNotSupported.into_soap_fault())
    }
}
