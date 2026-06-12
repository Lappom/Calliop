use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use super::capture::AudioError;

pub const DEFAULT_INPUT_DEVICE_ID: &str = "default";

#[derive(Debug, Clone, Serialize)]
pub struct InputDeviceInfo {
    pub id: String,
    pub label: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let default_name = default_device_name(&host);

    let mut devices = Vec::new();
    for device in host
        .input_devices()
        .map_err(|e| AudioError::DeviceEnumerate(e.to_string()))?
    {
        let name = device
            .name()
            .map_err(|e| AudioError::DeviceEnumerate(e.to_string()))?;
        let is_default = default_name.as_deref() == Some(name.as_str());
        devices.push(InputDeviceInfo {
            id: name.clone(),
            label: name,
            is_default,
        });
    }

    Ok(devices)
}

pub fn resolve_input_device(
    host: &cpal::Host,
    device_id: &str,
) -> Result<cpal::Device, AudioError> {
    if device_id.is_empty() || device_id == DEFAULT_INPUT_DEVICE_ID {
        return host.default_input_device().ok_or(AudioError::NoInputDevice);
    }

    for device in host
        .input_devices()
        .map_err(|e| AudioError::DeviceEnumerate(e.to_string()))?
    {
        let name = device
            .name()
            .map_err(|e| AudioError::DeviceEnumerate(e.to_string()))?;
        if name == device_id {
            return Ok(device);
        }
    }

    host.default_input_device().ok_or(AudioError::NoInputDevice)
}

fn default_device_name(host: &cpal::Host) -> Option<String> {
    host.default_input_device()
        .and_then(|device| device.name().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_default_input_device_does_not_panic() {
        let host = cpal::default_host();
        let result = resolve_input_device(&host, DEFAULT_INPUT_DEVICE_ID);
        if host.default_input_device().is_some() {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(AudioError::NoInputDevice)));
        }
    }

    #[test]
    fn list_input_devices_does_not_panic() {
        let devices = list_input_devices().expect("enumerate input devices");
        for device in &devices {
            assert!(!device.id.is_empty());
            assert!(!device.label.is_empty());
        }
        // cpal may report a default device whose name is absent from input_devices()
        // (common on headless Linux CI); resolve_input_device still handles "default".
    }
}
