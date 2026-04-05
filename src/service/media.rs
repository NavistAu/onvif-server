use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use soap_server::{SoapHandler, SoapFault};

use crate::error::OnvifError;
use crate::traits::MediaService;

pub struct MediaServiceHandler {
    pub(crate) svc: Arc<dyn MediaService>,
    pub(crate) xaddr: String,
}

impl MediaServiceHandler {
    pub fn new(svc: Arc<dyn MediaService>, xaddr: impl Into<String>) -> Self {
        Self { svc, xaddr: xaddr.into() }
    }
}

#[async_trait]
impl SoapHandler for MediaServiceHandler {
    async fn handle(&self, _body: Bytes) -> Result<Bytes, SoapFault> {
        Err(OnvifError::ActionNotSupported.into_soap_fault())
    }
}
