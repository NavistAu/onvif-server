use soap_server::fault::SoapFault;

/// ONVIF-specific error type that maps to SOAP faults with the ONVIF ter: namespace.
///
/// Every `into_soap_fault()` call produces a detail field containing
/// `xmlns:ter="http://www.onvif.org/ver10/error"` as required by the ONVIF spec.
#[derive(Debug, thiserror::Error)]
pub enum OnvifError {
    /// The requested action has not yet been implemented.
    #[error("Action not implemented")]
    NotImplemented,
    /// The caller supplied an invalid argument value.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    /// The device does not support the requested action.
    #[error("Action not supported")]
    ActionNotSupported,
}

impl OnvifError {
    /// Convert this error into a `SoapFault` with an ONVIF-namespaced detail element.
    ///
    /// The `xmlns:ter` declaration is embedded in the detail string because
    /// soap-server's envelope does not inject it automatically.
    pub fn into_soap_fault(self) -> SoapFault {
        match self {
            OnvifError::NotImplemented | OnvifError::ActionNotSupported => {
                let subcode = "ter:ActionNotSupported";
                let detail = format!(
                    r#"<ter:fault xmlns:ter="http://www.onvif.org/ver10/error"><ter:subcode>{subcode}</ter:subcode></ter:fault>"#
                );
                // The detail is well-formed XML, so route it via detail_xml: the SOAP 1.2
                // renderer only emits detail_xml inside <env:Detail> (the text `detail`
                // field is dropped as element-only, per F-3). Using `detail` here would
                // silently lose the ter: subcode on the wire for every ONVIF fault.
                SoapFault::receiver("Action not supported").with_detail_xml(detail)
            }
            OnvifError::InvalidArgument(msg) => {
                let subcode = "ter:InvalidArgVal";
                let detail = format!(
                    r#"<ter:fault xmlns:ter="http://www.onvif.org/ver10/error"><ter:subcode>{subcode}</ter:subcode></ter:fault>"#
                );
                SoapFault::sender(msg).with_detail_xml(detail)
            }
        }
    }
}

/// Convenience function returning `Err(OnvifError::NotImplemented)`.
///
/// Service trait implementations use this as a one-liner stub until the
/// real implementation is added.
pub fn not_implemented<T>() -> Result<T, OnvifError> {
    Err(OnvifError::NotImplemented)
}
