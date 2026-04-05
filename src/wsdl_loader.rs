use soap_server::{WsdlLoader, WsdlError};

/// Serves bundled ONVIF WSDL and XSD files from bytes embedded at compile time.
///
/// Official ONVIF WSDLs use relative schemaLocation paths like
/// `"../../../ver10/schema/onvif.xsd"` — the filename component is extracted
/// via `rsplit('/')` so callers can use either the bare name or a relative path.
pub struct EmbeddedWsdlLoader;

impl WsdlLoader for EmbeddedWsdlLoader {
    fn load(&self, location: &str) -> Result<Vec<u8>, WsdlError> {
        // Strip any leading path — WSDLs use relative paths like "../common/onvif.xsd"
        let filename = location.rsplit('/').next().unwrap_or(location);
        match filename {
            "devicemgmt.wsdl" => Ok(include_bytes!("../wsdl/devicemgmt.wsdl").to_vec()),
            "media.wsdl"      => Ok(include_bytes!("../wsdl/media.wsdl").to_vec()),
            "ptz.wsdl"        => Ok(include_bytes!("../wsdl/ptz.wsdl").to_vec()),
            "imaging.wsdl"    => Ok(include_bytes!("../wsdl/imaging.wsdl").to_vec()),
            // ONVIF events WSDL is named "event.wsdl" at source but stored as "events.wsdl"
            "events.wsdl" | "event.wsdl" => Ok(include_bytes!("../wsdl/events.wsdl").to_vec()),
            "onvif.xsd"       => Ok(include_bytes!("../wsdl/onvif.xsd").to_vec()),
            "common.xsd"      => Ok(include_bytes!("../wsdl/common.xsd").to_vec()),
            // W3C and OASIS external schemas imported by onvif.xsd.
            // Bundled as minimal stubs so ServerBuilder resolves the WSDL without network access.
            // URL → rsplit('/') → match key:
            //   https://www.w3.org/2005/05/xmlmime           → "xmlmime"
            //   https://www.w3.org/2003/05/soap-envelope     → "soap-envelope"
            //   http://docs.oasis-open.org/wsn/b-2.xsd       → "b-2.xsd"
            //   https://www.w3.org/2004/08/xop/include       → "include"
            "xmlmime" | "xmlmime.xsd" => Ok(include_bytes!("../wsdl/xmlmime.xsd").to_vec()),
            "soap-envelope" | "soap-envelope.xsd" => Ok(include_bytes!("../wsdl/soap-envelope.xsd").to_vec()),
            "b-2.xsd" => Ok(include_bytes!("../wsdl/wsn-b2.xsd").to_vec()),
            "include" | "xop-include.xsd" => Ok(include_bytes!("../wsdl/xop-include.xsd").to_vec()),
            other => Err(WsdlError::MalformedXml(format!("Unknown WSDL/XSD: {other}"))),
        }
    }
}
