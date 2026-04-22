// Server implementation — builder skeleton wired in plan 03; actual server start in phase 2
use std::collections::HashSet;
use std::sync::Arc;

use crate::service::device::DeviceServiceHandler;
use crate::service::events::EventServiceHandler;
use crate::service::imaging::ImagingServiceHandler;
use crate::service::media::MediaServiceHandler;
use crate::service::ptz::PTZServiceHandler;
use crate::traits::{DeviceService, EventService, ImagingService, MediaService, PTZService};
use crate::wsdl_loader::EmbeddedWsdlLoader;

/// Error returned by [`OnvifServerBuilder::build`] when required configuration is missing.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Required service not registered: {0}")]
    MissingRequiredService(String),
}

/// A built, configured ONVIF server handle.
///
/// Phase 1 stores all builder fields for Phase 2 to use when actually binding a port
/// and starting the soap-server. No network activity occurs in Phase 1.
pub struct OnvifServer {
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub device_service: Option<Arc<dyn DeviceService>>,
    pub media_service: Option<Arc<dyn MediaService>>,
    pub ptz_service: Option<Arc<dyn PTZService>>,
    pub imaging_service: Option<Arc<dyn ImagingService>>,
    pub event_service: Option<Arc<dyn EventService>>,
    pub auth_bypass: HashSet<String>,
    pub advertised_host: String,
}

impl OnvifServer {
    /// Create a new builder with default settings.
    ///
    /// Defaults: port 8080, `GetSystemDateAndTime` pre-registered as an auth bypass
    /// operation (per ONVIF spec — clock sync must work without credentials).
    pub fn builder() -> OnvifServerBuilder {
        OnvifServerBuilder::new()
    }

    /// Bind the configured port and start serving SOAP requests.
    ///
    /// This method does not return until the server is shut down.
    /// Requires a tokio async runtime (`#[tokio::main]` or `tokio::runtime::Runtime`).
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let device_svc = self
            .device_service
            .ok_or("device_service is required to call run()")?;

        let media_svc = self
            .media_service
            .ok_or("media_service is required to call run()")?;

        let ptz_svc = self
            .ptz_service
            .ok_or("ptz_service is required to call run()")?;

        let imaging_svc = self
            .imaging_service
            .ok_or("imaging_service is required to call run()")?;

        let event_svc = self
            .event_service
            .ok_or("event_service is required to call run()")?;

        let xaddr = format!(
            "http://{}:{}/onvif/device_service",
            self.advertised_host, self.port
        );
        let media_xaddr = format!(
            "http://{}:{}/onvif/media_service",
            self.advertised_host, self.port
        );
        let ptz_xaddr = format!(
            "http://{}:{}/onvif/ptz_service",
            self.advertised_host, self.port
        );
        let imaging_xaddr = format!(
            "http://{}:{}/onvif/imaging_service",
            self.advertised_host, self.port
        );
        let events_xaddr = format!(
            "http://{}:{}/onvif/events_service",
            self.advertised_host, self.port
        );

        let handler = DeviceServiceHandler::new(
            device_svc,
            xaddr,
            media_xaddr.clone(),
            ptz_xaddr,
            imaging_xaddr.clone(),
            events_xaddr.clone(),
        );

        let media_handler = MediaServiceHandler::new(media_svc, media_xaddr);
        let ptz_handler = PTZServiceHandler::new(ptz_svc);
        let imaging_handler = ImagingServiceHandler::new(imaging_svc);
        let events_handler = EventServiceHandler::new(event_svc, events_xaddr.clone());

        let username = self.username.clone();
        let password = self.password.clone();
        let username2 = self.username.clone();
        let password2 = self.password.clone();
        let username3 = self.username.clone();
        let password3 = self.password.clone();
        let username4 = self.username.clone();
        let password4 = self.password.clone();
        let username5 = self.username.clone();
        let password5 = self.password.clone();
        let auth_bypass = self.auth_bypass;

        let soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/devicemgmt.wsdl").to_vec(),
            EmbeddedWsdlLoader,
        )
        .path("/onvif/device_service")
        .default_handler(handler)
        .auth(move |user: &str| -> Option<String> {
            if Some(user) == username.as_deref() {
                password.clone()
            } else {
                None
            }
        })
        .auth_bypass(auth_bypass.into_iter())
        .build()
        .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

        let media_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/media.wsdl").to_vec(),
            EmbeddedWsdlLoader,
        )
        .path("/onvif/media_service")
        .default_handler(media_handler)
        .auth(move |user: &str| -> Option<String> {
            if Some(user) == username2.as_deref() {
                password2.clone()
            } else {
                None
            }
        })
        .auth_bypass(std::iter::empty::<String>())
        .build()
        .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

        let ptz_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/ptz.wsdl").to_vec(),
            EmbeddedWsdlLoader,
        )
        .path("/onvif/ptz_service")
        .default_handler(ptz_handler)
        .auth(move |user: &str| -> Option<String> {
            if Some(user) == username3.as_deref() {
                password3.clone()
            } else {
                None
            }
        })
        .auth_bypass(std::iter::empty::<String>())
        .build()
        .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

        let imaging_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/imaging.wsdl").to_vec(),
            EmbeddedWsdlLoader,
        )
        .path("/onvif/imaging_service")
        .default_handler(imaging_handler)
        .auth(move |user: &str| -> Option<String> {
            if Some(user) == username4.as_deref() {
                password4.clone()
            } else {
                None
            }
        })
        .auth_bypass(std::iter::empty::<String>())
        .build()
        .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

        let events_soap_svc = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
            include_bytes!("../wsdl/events.wsdl").to_vec(),
            EmbeddedWsdlLoader,
        )
        .path("/onvif/events_service")
        .default_handler(events_handler)
        .auth(move |user: &str| -> Option<String> {
            if Some(user) == username5.as_deref() {
                password5.clone()
            } else {
                None
            }
        })
        .auth_bypass(std::iter::empty::<String>())
        .build()
        .map_err(|e| format!("ServerBuilder::build failed: {e}"))?;

        let router = soap_svc
            .into_router()
            .merge(media_soap_svc.into_router())
            .merge(ptz_soap_svc.into_router())
            .merge(imaging_soap_svc.into_router())
            .merge(events_soap_svc.into_router());

        #[cfg(feature = "discovery")]
        {
            let disc_xaddr = format!(
                "http://{}:{}/onvif/device_service",
                self.advertised_host, self.port
            );
            tokio::spawn(async move {
                if let Err(e) = crate::discovery::run_discovery(disc_xaddr).await {
                    eprintln!("[discovery] task exited: {e}");
                }
            });
        }

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}

/// Builder for configuring and constructing an [`OnvifServer`].
///
/// Service registration, auth credentials, port, and auth bypass operations are
/// all set here. Phase 2 will call `build()` and then wire these fields into
/// `soap_server::ServerBuilder` to start the actual HTTP listener.
pub struct OnvifServerBuilder {
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub device_service: Option<Arc<dyn DeviceService>>,
    pub media_service: Option<Arc<dyn MediaService>>,
    pub ptz_service: Option<Arc<dyn PTZService>>,
    pub imaging_service: Option<Arc<dyn ImagingService>>,
    pub event_service: Option<Arc<dyn EventService>>,
    pub auth_bypass: HashSet<String>,
    pub advertised_host: String,
}

impl OnvifServerBuilder {
    fn new() -> Self {
        let mut auth_bypass = HashSet::new();
        // ONVIF spec requires GetSystemDateAndTime to be accessible without auth
        // so clients can synchronise their clocks before authenticating.
        auth_bypass.insert("GetSystemDateAndTime".to_string());

        Self {
            port: 8080,
            username: None,
            password: None,
            device_service: None,
            media_service: None,
            ptz_service: None,
            imaging_service: None,
            event_service: None,
            auth_bypass,
            advertised_host: "0.0.0.0".to_string(),
        }
    }

    /// Set the port the server will listen on. Defaults to 8080.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the host advertised in XAddrs for GetCapabilities, GetServices, and WS-Discovery.
    /// Real ONVIF clients need a routable address (e.g. "192.168.1.10"), not "0.0.0.0".
    /// Defaults to "0.0.0.0" for backward compatibility.
    pub fn advertised_host(mut self, host: &str) -> Self {
        self.advertised_host = host.to_string();
        self
    }

    /// Set the credentials used for WS-Security digest auth validation.
    ///
    /// Phase 2 will pass these to `soap_server::ServerBuilder::auth()` as a closure
    /// mapping usernames to their expected passwords.
    pub fn auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// Register a Device Management Service implementation.
    pub fn device_service(mut self, svc: impl DeviceService + 'static) -> Self {
        self.device_service = Some(Arc::new(svc));
        self
    }

    /// Register a Media Service implementation.
    pub fn media_service(mut self, svc: impl MediaService + 'static) -> Self {
        self.media_service = Some(Arc::new(svc));
        self
    }

    /// Register a PTZ Service implementation.
    pub fn ptz_service(mut self, svc: impl PTZService + 'static) -> Self {
        self.ptz_service = Some(Arc::new(svc));
        self
    }

    /// Register an Imaging Service implementation.
    pub fn imaging_service(mut self, svc: impl ImagingService + 'static) -> Self {
        self.imaging_service = Some(Arc::new(svc));
        self
    }

    /// Register an Event Service implementation.
    pub fn event_service(mut self, svc: impl EventService + 'static) -> Self {
        self.event_service = Some(Arc::new(svc));
        self
    }

    /// Accessor for the auth bypass operation set. Used in tests and Phase 2 wiring.
    pub fn auth_bypass_set(&self) -> &HashSet<String> {
        &self.auth_bypass
    }

    /// Build the configured [`OnvifServer`].
    ///
    /// Phase 1: validates configuration and returns a server handle with all fields stored.
    /// Phase 2 will extend this to call `soap_server::ServerBuilder` and bind the port.
    pub fn build(self) -> Result<OnvifServer, BuildError> {
        Ok(OnvifServer {
            port: self.port,
            username: self.username,
            password: self.password,
            device_service: self.device_service,
            media_service: self.media_service,
            ptz_service: self.ptz_service,
            imaging_service: self.imaging_service,
            event_service: self.event_service,
            auth_bypass: self.auth_bypass,
            advertised_host: self.advertised_host,
        })
    }
}
