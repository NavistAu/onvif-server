// examples/virtual_ptz.rs
//
// Virtual PTZ ONVIF camera — a minimal in-memory PTZ camera that implements
// all five required service traits (DeviceService, MediaService, PTZService,
// ImagingService, EventService).
//
// Usage:
//   cargo run --example virtual_ptz
//
// Then connect any ONVIF client (e.g., ONVIF Device Manager, VLC, or Frigate)
// to http://<host>:8080/onvif/device_service using credentials admin/admin.
//
// The server exposes:
//   - /onvif/device_service  — Device management
//   - /onvif/media_service   — Media profiles and stream URIs
//   - /onvif/ptz_service     — Full PTZ control with in-memory preset storage
//   - /onvif/imaging_service — Imaging settings
//   - /onvif/events_service  — Event subscriptions
//
// PTZ presets are stored in memory and lost on restart.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use onvif_server::generated::DeviceInfo;
use onvif_server::{
    DeviceService, EventService, ImagingService, ImagingSettings, MediaService, OnvifError,
    PTZPreset, PTZService, PTZStatusResult,
};

// ---------------------------------------------------------------------------
// VirtualPTZ — shared state via Arc<Mutex<_>>
// ---------------------------------------------------------------------------

/// A virtual PTZ camera with in-memory preset storage.
///
/// Uses Arc internally so it can be cheaply cloned and registered for all
/// three service slots (device, media, ptz) while sharing the same state.
#[derive(Clone)]
struct VirtualPTZ {
    presets: Arc<Mutex<HashMap<String, String>>>,
    preset_counter: Arc<Mutex<u32>>,
}

impl VirtualPTZ {
    fn new() -> Self {
        Self {
            presets: Arc::new(Mutex::new(HashMap::new())),
            preset_counter: Arc::new(Mutex::new(0)),
        }
    }
}

// ---------------------------------------------------------------------------
// DeviceService
// ---------------------------------------------------------------------------

#[async_trait]
impl DeviceService for VirtualPTZ {
    async fn get_device_information(&self) -> Result<DeviceInfo, OnvifError> {
        Ok(DeviceInfo {
            manufacturer: "Virtual".to_string(),
            model: "VirtualPTZ".to_string(),
            firmware_version: "1.0".to_string(),
            serial_number: "0000".to_string(),
            hardware_id: "virtual-hw-0".to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// MediaService
// ---------------------------------------------------------------------------

#[async_trait]
impl MediaService for VirtualPTZ {
    async fn get_stream_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok("rtsp://127.0.0.1:8554/stream".to_string())
    }

    async fn get_snapshot_uri(&self, _profile_token: &str) -> Result<String, OnvifError> {
        Ok("http://127.0.0.1:8080/snapshot.jpg".to_string())
    }
}

// ---------------------------------------------------------------------------
// PTZService
// ---------------------------------------------------------------------------

#[async_trait]
impl PTZService for VirtualPTZ {
    async fn relative_move(
        &self,
        profile_token: &str,
        pan: f32,
        tilt: f32,
        zoom: f32,
    ) -> Result<(), OnvifError> {
        println!(
            "[VirtualPTZ] relative_move: profile={profile_token} pan={pan:.3} tilt={tilt:.3} zoom={zoom:.3}"
        );
        Ok(())
    }

    async fn absolute_move(
        &self,
        profile_token: &str,
        pan: f32,
        tilt: f32,
        zoom: f32,
    ) -> Result<(), OnvifError> {
        println!(
            "[VirtualPTZ] absolute_move: profile={profile_token} pan={pan:.3} tilt={tilt:.3} zoom={zoom:.3}"
        );
        Ok(())
    }

    async fn continuous_move(
        &self,
        profile_token: &str,
        pan: f32,
        tilt: f32,
        zoom: f32,
    ) -> Result<(), OnvifError> {
        println!(
            "[VirtualPTZ] continuous_move: profile={profile_token} pan={pan:.3} tilt={tilt:.3} zoom={zoom:.3}"
        );
        Ok(())
    }

    async fn stop(
        &self,
        profile_token: &str,
        pan_tilt: bool,
        zoom: bool,
    ) -> Result<(), OnvifError> {
        println!("[VirtualPTZ] stop: profile={profile_token} pan_tilt={pan_tilt} zoom={zoom}");
        Ok(())
    }

    async fn get_status(&self, profile_token: &str) -> Result<PTZStatusResult, OnvifError> {
        println!("[VirtualPTZ] get_status: profile={profile_token}");
        Ok(PTZStatusResult {
            pan_tilt_moving: false,
            zoom_moving: false,
        })
    }

    async fn get_presets(&self, profile_token: &str) -> Result<Vec<PTZPreset>, OnvifError> {
        let guard = self.presets.lock().unwrap();
        let presets: Vec<PTZPreset> = guard
            .iter()
            .map(|(token, name)| PTZPreset {
                token: token.clone(),
                name: name.clone(),
            })
            .collect();
        println!(
            "[VirtualPTZ] get_presets: profile={profile_token} found={} presets",
            presets.len()
        );
        Ok(presets)
    }

    async fn set_preset(
        &self,
        profile_token: &str,
        preset_name: Option<&str>,
        preset_token: Option<&str>,
    ) -> Result<String, OnvifError> {
        let token = if let Some(t) = preset_token {
            t.to_string()
        } else {
            let mut counter = self.preset_counter.lock().unwrap();
            *counter += 1;
            format!("preset_{}", *counter)
        };
        let name = preset_name.unwrap_or(&token).to_string();
        self.presets
            .lock()
            .unwrap()
            .insert(token.clone(), name.clone());
        println!("[VirtualPTZ] set_preset: profile={profile_token} token={token} name={name}");
        Ok(token)
    }

    async fn goto_preset(&self, profile_token: &str, preset_token: &str) -> Result<(), OnvifError> {
        println!(
            "[VirtualPTZ] goto_preset: profile={profile_token} moving to preset={preset_token}"
        );
        Ok(())
    }

    async fn remove_preset(
        &self,
        profile_token: &str,
        preset_token: &str,
    ) -> Result<(), OnvifError> {
        self.presets.lock().unwrap().remove(preset_token);
        println!(
            "[VirtualPTZ] remove_preset: profile={profile_token} removed token={preset_token}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ImagingService
// ---------------------------------------------------------------------------

#[async_trait]
impl ImagingService for VirtualPTZ {
    async fn get_imaging_settings(
        &self,
        _video_source_token: String,
    ) -> Result<ImagingSettings, OnvifError> {
        Ok(ImagingSettings {
            brightness: Some(50.0),
            contrast: Some(50.0),
            sharpness: Some(50.0),
            ..Default::default()
        })
    }
}

// ---------------------------------------------------------------------------
// EventService
// ---------------------------------------------------------------------------

#[async_trait]
impl EventService for VirtualPTZ {}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cam = VirtualPTZ::new();

    let server = onvif_server::OnvifServer::builder()
        .port(8080)
        .auth("admin", "admin")
        .device_service(cam.clone())
        .media_service(cam.clone())
        .ptz_service(cam.clone())
        .imaging_service(cam.clone())
        .event_service(cam)
        .build()?;

    println!("Virtual PTZ ONVIF server running on :8080");
    println!("  Device service:  http://0.0.0.0:8080/onvif/device_service");
    println!("  Media service:   http://0.0.0.0:8080/onvif/media_service");
    println!("  PTZ service:     http://0.0.0.0:8080/onvif/ptz_service");
    println!("  Imaging service: http://0.0.0.0:8080/onvif/imaging_service");
    println!("  Events service:  http://0.0.0.0:8080/onvif/events_service");
    println!("  Credentials:     admin / admin");

    server.run().await
}
