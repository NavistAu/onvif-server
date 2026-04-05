// Error types — implemented in task 2
use soap_server::fault::{FaultCode, SoapFault};

#[derive(Debug, thiserror::Error)]
pub enum OnvifError {
    #[error("Action not implemented")]
    NotImplemented,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Action not supported")]
    ActionNotSupported,
}

impl OnvifError {
    pub fn into_soap_fault(self) -> SoapFault {
        todo!("implemented in task 2")
    }
}

pub fn not_implemented() -> Result<(), OnvifError> {
    Err(OnvifError::NotImplemented)
}
