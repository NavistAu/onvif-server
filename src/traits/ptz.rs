use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};
use crate::generated::{PTZStatusResult, PTZPreset};

/// ONVIF PTZ Service (Profile S) — control operations only.
///
/// Discovery operations (GetNodes, GetNode, GetConfigurations, GetConfiguration,
/// GetConfigurationOptions, GetServiceCapabilities) are handler-internal and return
/// static XML — they are NOT on this trait.
///
/// All methods default to `not_implemented()` except `get_presets` which defaults to
/// returning an empty list. Store as `Arc<dyn PTZService>`.
#[async_trait]
pub trait PTZService: Send + Sync + 'static {
    /// Performs a relative pan/tilt/zoom movement.
    async fn relative_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Performs an absolute pan/tilt/zoom movement.
    async fn absolute_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Performs a continuous pan/tilt/zoom movement.
    async fn continuous_move(
        &self,
        _profile_token: &str,
        _pan: f32,
        _tilt: f32,
        _zoom: f32,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Stops any ongoing PTZ movement.
    async fn stop(
        &self,
        _profile_token: &str,
        _pan_tilt: bool,
        _zoom: bool,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the current PTZ position and move status.
    async fn get_status(
        &self,
        _profile_token: &str,
    ) -> Result<PTZStatusResult, OnvifError> {
        not_implemented()
    }

    /// Returns all saved preset positions. Defaults to empty list.
    async fn get_presets(
        &self,
        _profile_token: &str,
    ) -> Result<Vec<PTZPreset>, OnvifError> {
        Ok(vec![])
    }

    /// Moves the camera to a saved preset position.
    async fn goto_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Saves the current position as a named preset. Returns the preset token.
    async fn set_preset(
        &self,
        _profile_token: &str,
        _preset_name: Option<&str>,
        _preset_token: Option<&str>,
    ) -> Result<String, OnvifError> {
        not_implemented()
    }

    /// Deletes a saved preset.
    async fn remove_preset(
        &self,
        _profile_token: &str,
        _preset_token: &str,
    ) -> Result<(), OnvifError> {
        not_implemented()
    }
}
