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

/// Error returned by [`OnvifServer::run`].
///
/// Distinguishes between startup/configuration failures and I/O failures at bind time.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The server failed to bind or serve on the configured port (I/O error).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A required service was not registered (validated again at run time for services
    /// beyond `device_service`, which is checked at build time).
    #[error("Startup error: {0}")]
    Startup(String),
}

/// A built, configured ONVIF server handle.
///
/// Phase 1 stores all builder fields for Phase 2 to use when actually binding a port
/// and starting the soap-server. No network activity occurs in Phase 1.
///
/// Fields are intentionally `pub(crate)` to prevent consumers from bypassing the
/// builder API or accessing credentials directly. Use the provided accessor methods
/// for fields that consumers legitimately need.
pub struct OnvifServer {
    pub(crate) port: u16,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) device_service: Option<Arc<dyn DeviceService>>,
    pub(crate) media_service: Option<Arc<dyn MediaService>>,
    pub(crate) ptz_service: Option<Arc<dyn PTZService>>,
    pub(crate) imaging_service: Option<Arc<dyn ImagingService>>,
    pub(crate) event_service: Option<Arc<dyn EventService>>,
    pub(crate) auth_bypass: HashSet<String>,
    pub(crate) advertised_host: String,
    /// WS-Discovery EndpointReference UUID for this device (F-7).
    ///
    /// ONVIF WS-Discovery requires the EndpointReference Address to be a stable
    /// per-device identity across all discovery cycles.  This UUID is fixed for the
    /// lifetime of the server, so every ProbeMatch within a single process run
    /// carries the same identity.  When explicitly set via
    /// [`OnvifServerBuilder::discovery_uuid`], that UUID is used verbatim.  When
    /// unset, a random UUID-v4 is generated once when the builder is created — it is
    /// stable across discovery cycles but **not** across restarts.  Supply a fixed
    /// UUID (e.g. derived from a hardware ID or stored config) for identity that
    /// survives restarts.
    pub(crate) discovery_uuid: uuid::Uuid,
}

impl OnvifServer {
    /// Returns the port this server is configured to listen on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the advertised host used in XAddrs for WS-Discovery and capabilities.
    pub fn advertised_host(&self) -> &str {
        &self.advertised_host
    }

    /// Returns the WS-Discovery EndpointReference UUID for this device.
    ///
    /// ONVIF conformance (F-7) requires this to be identical across all discovery
    /// cycles, which it is for the lifetime of the server.  Set it explicitly via
    /// [`OnvifServerBuilder::discovery_uuid`] for an identity that also survives
    /// restarts; otherwise a random UUID-v4 is generated once at builder creation.
    pub fn discovery_uuid(&self) -> uuid::Uuid {
        self.discovery_uuid
    }

    /// Returns the configured username, if any.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Create a new builder with default settings.
    ///
    /// Defaults: port 8080, `GetSystemDateAndTime` pre-registered as an auth bypass
    /// operation (per ONVIF spec — clock sync must work without credentials).
    pub fn builder() -> OnvifServerBuilder {
        OnvifServerBuilder::new()
    }

    /// Build the merged axum `Router` for all registered services (device + any registered
    /// media/ptz/imaging/events), WITHOUT binding a port or starting the WS-Discovery UDP task.
    /// Used by `run()` and by in-process harnesses/tests (axum_test).
    pub fn into_router(self) -> Result<axum::Router, RunError> {
        let device_svc = self
            .device_service
            .ok_or_else(|| RunError::Startup("device_service is required".into()))?;

        let xaddr = format!(
            "http://{}:{}/onvif/device_service",
            self.advertised_host, self.port
        );

        // Build optional XAddrs — only Some when the corresponding service is registered.
        let media_xaddr = self.media_service.as_ref().map(|_| {
            format!(
                "http://{}:{}/onvif/media_service",
                self.advertised_host, self.port
            )
        });
        let ptz_xaddr = self.ptz_service.as_ref().map(|_| {
            format!(
                "http://{}:{}/onvif/ptz_service",
                self.advertised_host, self.port
            )
        });
        let imaging_xaddr = self.imaging_service.as_ref().map(|_| {
            format!(
                "http://{}:{}/onvif/imaging_service",
                self.advertised_host, self.port
            )
        });
        let events_xaddr = self.event_service.as_ref().map(|_| {
            format!(
                "http://{}:{}/onvif/events_service",
                self.advertised_host, self.port
            )
        });

        let handler = DeviceServiceHandler::new(
            device_svc,
            xaddr,
            media_xaddr.clone().unwrap_or_default(),
            ptz_xaddr.clone().unwrap_or_default(),
            imaging_xaddr.clone().unwrap_or_default(),
            events_xaddr.clone().unwrap_or_default(),
        );

        let auth_bypass = self.auth_bypass;
        let credentials = self
            .username
            .as_ref()
            .zip(self.password.as_ref())
            .map(|(u, p)| (u.clone(), p.clone()));

        // Helper macro: builds a soap_server::ServerBuilder for a given WSDL/path,
        // attaches auth only when credentials are configured, then calls .build().
        macro_rules! build_service {
            ($wsdl_bytes:expr, $path:expr, $handler:expr, $bypass_iter:expr) => {{
                let mut b = soap_server::ServerBuilder::from_wsdl_bytes_with_loader(
                    $wsdl_bytes.to_vec(),
                    EmbeddedWsdlLoader,
                )
                .path($path)
                .default_handler($handler)
                .auth_bypass($bypass_iter);

                if let Some((ref u, ref p)) = credentials {
                    let u = u.clone();
                    let p = p.clone();
                    b = b.auth(move |user: &str| -> Option<String> {
                        if user == u {
                            Some(p.clone())
                        } else {
                            None
                        }
                    });
                }

                b.build()
                    .map_err(|e| RunError::Startup(format!("ServerBuilder::build failed: {e}")))?
            }};
        }

        let soap_svc = build_service!(
            include_bytes!("../wsdl/devicemgmt.wsdl"),
            "/onvif/device_service",
            handler,
            auth_bypass.into_iter()
        );

        let mut router = soap_svc.into_router();

        // Mount optional services — only when registered.
        if let Some(media_svc) = self.media_service {
            let media_handler =
                MediaServiceHandler::new(media_svc, media_xaddr.as_deref().unwrap_or_default());
            let media_soap_svc = build_service!(
                include_bytes!("../wsdl/media.wsdl"),
                "/onvif/media_service",
                media_handler,
                std::iter::empty::<String>()
            );
            router = router.merge(media_soap_svc.into_router());
        }

        if let Some(ptz_svc) = self.ptz_service {
            let ptz_handler = PTZServiceHandler::new(ptz_svc);
            let ptz_soap_svc = build_service!(
                include_bytes!("../wsdl/ptz.wsdl"),
                "/onvif/ptz_service",
                ptz_handler,
                std::iter::empty::<String>()
            );
            router = router.merge(ptz_soap_svc.into_router());
        }

        if let Some(imaging_svc) = self.imaging_service {
            let imaging_handler = ImagingServiceHandler::new(imaging_svc);
            let imaging_soap_svc = build_service!(
                include_bytes!("../wsdl/imaging.wsdl"),
                "/onvif/imaging_service",
                imaging_handler,
                std::iter::empty::<String>()
            );
            router = router.merge(imaging_soap_svc.into_router());
        }

        if let Some(event_svc) = self.event_service {
            let events_xaddr_str = events_xaddr.as_deref().unwrap_or_default().to_string();
            let events_handler = EventServiceHandler::new(event_svc, events_xaddr_str);
            let events_soap_svc = build_service!(
                include_bytes!("../wsdl/events.wsdl"),
                "/onvif/events_service",
                events_handler,
                std::iter::empty::<String>()
            );
            router = router.merge(events_soap_svc.into_router());
        }

        Ok(router)
    }

    /// Bind the configured port and start serving SOAP requests.
    ///
    /// This method does not return until the server is shut down.
    /// Requires a tokio async runtime (`#[tokio::main]` or `tokio::runtime::Runtime`).
    ///
    /// # Auth behaviour
    ///
    /// If the builder was configured with `.auth(username, password)`, WS-Security
    /// UsernameToken authentication is enforced on all non-bypassed operations.
    /// If `.auth()` was NOT called, the server runs **unauthenticated** — all
    /// operations are accessible without credentials.
    ///
    /// # Service optionality
    ///
    /// Only `device_service` is required (checked at [`OnvifServerBuilder::build`] time).
    /// Media, PTZ, Imaging, and Events services are optional; their routes are only
    /// mounted and their capabilities only advertised when they are registered.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Startup`] if `device_service` is somehow absent at run time.
    /// Returns [`RunError::Io`] if the TCP listener fails to bind or serve.
    pub async fn run(self) -> Result<(), RunError> {
        // Capture port and xaddr BEFORE consuming self via into_router().
        let port = self.port;
        let disc_xaddr = format!(
            "http://{}:{}/onvif/device_service",
            self.advertised_host, self.port
        );
        // The device's WS-Discovery EndpointReference UUID is a STABLE identity for the
        // device's lifetime (set via the builder, else a per-build default) — never
        // regenerated per probe.
        let disc_uuid = self.discovery_uuid;

        #[cfg(feature = "discovery")]
        {
            let xaddr_for_disc = disc_xaddr.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::discovery::run_discovery_with_uuid(xaddr_for_disc, disc_uuid).await
                {
                    eprintln!("[discovery] task exited: {e}");
                }
            });
        }
        let _ = disc_uuid;
        // Suppress unused-variable warning when discovery feature is off.
        let _ = disc_xaddr;

        let router = self.into_router()?;

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}

/// Builder for configuring and constructing an [`OnvifServer`].
///
/// Service registration, auth credentials, port, and auth bypass operations are
/// all set here. Fields are `pub(crate)` — use the builder methods to configure
/// the server.
pub struct OnvifServerBuilder {
    pub(crate) port: u16,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) device_service: Option<Arc<dyn DeviceService>>,
    pub(crate) media_service: Option<Arc<dyn MediaService>>,
    pub(crate) ptz_service: Option<Arc<dyn PTZService>>,
    pub(crate) imaging_service: Option<Arc<dyn ImagingService>>,
    pub(crate) event_service: Option<Arc<dyn EventService>>,
    pub(crate) auth_bypass: HashSet<String>,
    pub(crate) advertised_host: String,
    pub(crate) discovery_uuid: uuid::Uuid,
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
            discovery_uuid: uuid::Uuid::new_v4(),
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
    /// When called, WS-Security UsernameToken authentication is enforced on all
    /// non-bypassed operations during [`OnvifServer::run`]. When NOT called, the
    /// server runs **unauthenticated** — all operations are accessible without
    /// credentials.
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
    ///
    /// Optional: if not registered, the media route is not mounted and media
    /// capabilities are not advertised.
    pub fn media_service(mut self, svc: impl MediaService + 'static) -> Self {
        self.media_service = Some(Arc::new(svc));
        self
    }

    /// Register a PTZ Service implementation.
    ///
    /// Optional: if not registered, the PTZ route is not mounted and PTZ
    /// capabilities are not advertised.
    pub fn ptz_service(mut self, svc: impl PTZService + 'static) -> Self {
        self.ptz_service = Some(Arc::new(svc));
        self
    }

    /// Register an Imaging Service implementation.
    ///
    /// Optional: if not registered, the imaging route is not mounted and imaging
    /// capabilities are not advertised.
    pub fn imaging_service(mut self, svc: impl ImagingService + 'static) -> Self {
        self.imaging_service = Some(Arc::new(svc));
        self
    }

    /// Register an Event Service implementation.
    ///
    /// Optional: if not registered, the events route is not mounted and events
    /// capabilities are not advertised.
    pub fn event_service(mut self, svc: impl EventService + 'static) -> Self {
        self.event_service = Some(Arc::new(svc));
        self
    }

    /// Override the stable WS-Discovery EndpointReference UUID for this device.
    ///
    /// When not called, the builder defaults to a random UUID-v4.  Callers that
    /// need a deterministic identity across restarts should supply a stable UUID
    /// here (e.g. derived from hardware ID or stored configuration).
    pub fn discovery_uuid(mut self, uuid: uuid::Uuid) -> Self {
        self.discovery_uuid = uuid;
        self
    }

    /// Accessor for the auth bypass operation set. Used in tests and Phase 2 wiring.
    pub fn auth_bypass_set(&self) -> &HashSet<String> {
        &self.auth_bypass
    }

    /// Build the configured [`OnvifServer`].
    ///
    /// Returns `Err(BuildError::MissingRequiredService("device_service"))` if no
    /// device service has been registered. `device_service` is required by the ONVIF
    /// spec — it provides `GetSystemDateAndTime` and core device management operations.
    ///
    /// All other services (media, PTZ, imaging, events) are optional. When omitted,
    /// their routes are not mounted at run time and their capabilities are not
    /// advertised in `GetCapabilities` / `GetServices`.
    pub fn build(self) -> Result<OnvifServer, BuildError> {
        if self.device_service.is_none() {
            return Err(BuildError::MissingRequiredService(
                "device_service".to_string(),
            ));
        }
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
            discovery_uuid: self.discovery_uuid,
        })
    }
}
