// examples/minimal_device.rs
//
// The smallest ONVIF device that is actually *usable* by a real client.
//
// Unlike an empty `impl DeviceService for MyCamera {}` (which builds but faults
// on the first real request), this device implements the handful of operations a
// client needs to enumerate the device and start a stream:
//
//   - GetDeviceInformation  (manufacturer/model/...) — no working default
//   - GetStreamUri          (the RTSP URL)           — no working default
//   - GetSnapshotUri        (the JPEG URL)           — no working default
//
// Everything else uses the framework defaults: GetSystemDateAndTime, GetScopes,
// GetHostname, GetCapabilities, GetServices, and a single 1920x1080 H264 media
// profile from MediaService::profiles().
//
// Usage:
//   cargo run --example minimal_device
//
// Then point an ONVIF client at http://<host>:8080/onvif/device_service with
// credentials admin / password, or query it directly (see book: Quick Start).

use async_trait::async_trait;
use onvif_server::{DeviceInfo, DeviceService, MediaService, OnvifError, OnvifServer};

/// A minimal, read-only ONVIF camera backed by a fixed RTSP/HTTP endpoint.
#[derive(Clone)]
struct MinimalCamera {
    /// Base host clients should connect to for media (the camera's routable IP).
    media_host: String,
}

#[async_trait]
impl DeviceService for MinimalCamera {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "Example Corp".to_string(),
            model: "Minimal-1".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "SN-0001".to_string(),
            hardware_id: "minimal-hw-1".to_string(),
        })
    }
    // get_scopes, get_hostname, get_system_date_and_time use the working defaults.
    // get_network_interfaces is left as default (faults) — most clients never call it.
}

#[async_trait]
impl MediaService for MinimalCamera {
    // profiles() uses the default single 1920x1080 H264 "MainProfile".

    async fn get_stream_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("rtsp://{}:8554/stream", self.media_host))
    }

    async fn get_snapshot_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok(format!("http://{}:8080/snapshot.jpg", self.media_host))
    }
}

#[tokio::main]
async fn main() {
    // In a real deployment this is the camera's LAN IP — the address clients use
    // to reach both the ONVIF service and the advertised stream/snapshot URIs.
    let host = "127.0.0.1";
    let cam = MinimalCamera {
        media_host: host.to_string(),
    };

    println!("Minimal ONVIF device on :8080");
    println!("  Device service: http://{host}:8080/onvif/device_service");
    println!("  Media service:  http://{host}:8080/onvif/media_service");
    println!("  Credentials:    admin / password");

    OnvifServer::builder()
        .port(8080)
        .advertised_host(host)
        .device_service(cam.clone())
        .media_service(cam)
        .auth("admin", "password")
        .build()
        .expect("build failed")
        .run()
        .await
        .expect("server error");
}
