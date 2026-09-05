//! Optional DXGI device manager for MF hardware decode (DXVA).

use super::PlayerError;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Multithread, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Media::MediaFoundation::{IMFDXGIDeviceManager, MFCreateDXGIDeviceManager};

pub struct DxvaContext {
    pub manager: IMFDXGIDeviceManager,
    _device: ID3D11Device,
}

impl DxvaContext {
    pub fn try_create() -> Result<Self, PlayerError> {
        unsafe {
            let mut device = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
            .map_err(|e| PlayerError::Message(format!("D3D11CreateDevice: {e}")))?;
            let device = device.ok_or_else(|| PlayerError::Message("null D3D11 device".into()))?;

            if let Ok(mt) = device.cast::<ID3D11Multithread>() {
                let _ = mt.SetMultithreadProtected(true);
            }

            let mut token = 0u32;
            let mut manager = None;
            MFCreateDXGIDeviceManager(&mut token, &mut manager)
                .map_err(|e| PlayerError::Message(format!("MFCreateDXGIDeviceManager: {e}")))?;
            let manager =
                manager.ok_or_else(|| PlayerError::Message("null DXGI device manager".into()))?;
            manager
                .ResetDevice(&device, token)
                .map_err(|e| PlayerError::Message(format!("ResetDevice: {e}")))?;

            Ok(Self {
                manager,
                _device: device,
            })
        }
    }
}
