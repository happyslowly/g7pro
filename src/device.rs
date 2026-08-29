use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};
use std::ffi::CString;

pub(crate) const VENDOR_ID: u16 = 0x3537;

#[derive(Debug)]
struct DeviceProfile {
    usage_page: u16,
    usage: u16,
}

// The controller's private vendor interface: the Usage Page 0xFFF0 / Usage 0x40
// collection found in the report_descriptor, which carries report 0x10 / 0x12
// (status) / 0x0f (heartbeat, commands).
// product_id changes depending on whether the receiver is idle or paired to a
// controller (0575 <-> 10c6), so we can't match on product_id — only this
// usage_page/usage combination stays stable across both states.
const DEVICE_PROFILE: DeviceProfile = DeviceProfile {
    usage_page: 0xfff0,
    usage: 0x0040,
};

#[derive(Debug)]
pub(crate) struct G7ProDeviceInfo {
    path: CString,
    product_id: u16,
    product_name: Option<String>,
}

impl G7ProDeviceInfo {
    fn from(info: &DeviceInfo) -> Self {
        G7ProDeviceInfo {
            path: info.path().to_owned(),
            product_id: info.product_id(),
            product_name: info.product_string().map(str::to_string),
        }
    }

    pub(crate) fn open(&self, api: &HidApi) -> Result<HidDevice, HidError> {
        api.open_path(self.path.as_c_str())
    }

    pub(crate) fn product_id(&self) -> u16 {
        self.product_id
    }

    pub(crate) fn product_name(&self) -> Option<&str> {
        self.product_name.as_deref()
    }
}

pub(crate) fn get_devices_by_vendor(api: &HidApi, vendor_id: u16) -> Vec<G7ProDeviceInfo> {
    let mut devices = Vec::new();

    for device in api.device_list() {
        if device.vendor_id() != vendor_id {
            continue;
        }

        if device.usage_page() == DEVICE_PROFILE.usage_page && device.usage() == DEVICE_PROFILE.usage
        {
            devices.push(G7ProDeviceInfo::from(device));
        }
    }

    devices
}

pub(crate) fn find_g7pro(api: &HidApi) -> Option<G7ProDeviceInfo> {
    get_devices_by_vendor(api, VENDOR_ID).into_iter().next()
}
