//! Controlled ONVIF server — deterministic fixture on all services (spec §7).
//!
//! Builds an `OnvifServer` from `ControlledCamera` with admin/admin credentials,
//! binds on `0.0.0.0:8080`, and advertises `controlled-onvif:8080` in XAddrs.
//!
//! `GetSystemDateAndTime` is auth-bypassed automatically by `OnvifServer::builder()`
//! (hardwired in `OnvifServerBuilder::new()` per ONVIF spec §9 — no extra builder
//! call is needed).
//!
//! The advertised host can be overridden at runtime via the `ONVIF_HOST` environment
//! variable (useful for Layer-2 container deployments where the container hostname
//! differs from the default).
//!
//! # Usage
//! ```sh
//! cargo run --bin controlled_onvif_server
//! # or with a custom advertised host:
//! ONVIF_HOST=192.168.1.42:8080 cargo run --bin controlled_onvif_server
//! ```

use onvif_crossref::fixture::ControlledCamera;
use onvif_server::OnvifServer;

const DEFAULT_HOST: &str = "controlled-onvif:8080";
const PORT: u16 = 8080;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let advertised_host = std::env::var("ONVIF_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());

    let cam = ControlledCamera::with_host(&advertised_host);

    let server = OnvifServer::builder()
        .port(PORT)
        .advertised_host(&advertised_host)
        .auth("admin", "admin")
        // GetSystemDateAndTime is pre-registered as auth-bypassed in builder::new()
        // per ONVIF spec; no additional call required.
        .device_service(cam.clone())
        .media_service(cam.clone())
        .ptz_service(cam.clone())
        .imaging_service(cam.clone())
        .event_service(cam)
        .build()?;

    eprintln!("[controlled-onvif] Starting on 0.0.0.0:{PORT}");
    eprintln!("[controlled-onvif] Advertised host: {advertised_host}");
    eprintln!("[controlled-onvif] Device service:  http://{advertised_host}/onvif/device_service");
    eprintln!("[controlled-onvif] Media service:   http://{advertised_host}/onvif/media_service");
    eprintln!("[controlled-onvif] PTZ service:     http://{advertised_host}/onvif/ptz_service");
    eprintln!("[controlled-onvif] Imaging service: http://{advertised_host}/onvif/imaging_service");
    eprintln!("[controlled-onvif] Events service:  http://{advertised_host}/onvif/events_service");
    eprintln!("[controlled-onvif] Credentials:     admin / admin");

    Ok(server.run().await?)
}
