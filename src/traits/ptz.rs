use async_trait::async_trait;
use crate::error::{OnvifError, not_implemented};

/// ONVIF PTZ Service (Profile S).
///
/// All methods default to `not_implemented()`. Object-safe: store as `Arc<dyn PTZService>`.
#[async_trait]
pub trait PTZService: Send + Sync + 'static {
    /// Returns all PTZ nodes available on this device.
    async fn get_nodes(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns all PTZ configurations.
    async fn get_configurations(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Performs a relative pan/tilt/zoom movement.
    async fn relative_move(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Performs an absolute pan/tilt/zoom movement.
    async fn absolute_move(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Performs a continuous pan/tilt/zoom movement.
    async fn continuous_move(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Stops any ongoing PTZ movement.
    async fn stop(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns the current PTZ position and move status.
    async fn get_status(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Moves the camera to a saved preset position.
    async fn goto_preset(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Returns all saved preset positions.
    async fn get_presets(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Saves the current position as a named preset.
    async fn set_preset(&self) -> Result<(), OnvifError> {
        not_implemented()
    }

    /// Deletes a saved preset.
    async fn remove_preset(&self) -> Result<(), OnvifError> {
        not_implemented()
    }
}
