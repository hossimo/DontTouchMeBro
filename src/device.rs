//! Device access through the Configuration Manager (cfgmgr32) and SetupAPI —
//! the same APIs Device Manager and `pnputil` use, without WMI/COM.
//!
//! Reading device state works unelevated; enabling or disabling needs
//! Administrator (the app manifest requests it).

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_DEVNODE_STATUS_FLAGS, CM_DRP_CLASS, CM_DRP_DEVICEDESC, CM_DRP_FRIENDLYNAME, CM_DRP_MFG,
    CM_Get_DevNode_Registry_PropertyW, CM_Get_DevNode_Status, CM_Get_Device_IDW,
    CM_LOCATE_DEVNODE_FLAGS, CM_LOCATE_DEVNODE_NORMAL, CM_LOCATE_DEVNODE_PHANTOM,
    CM_Locate_DevNodeW, CM_MapCrToWin32Err, CM_PROB, CONFIGRET, CR_BUFFER_SMALL,
    CR_NO_SUCH_DEVNODE, CR_SUCCESS, DI_NEEDREBOOT, DI_NEEDRESTART, DICS_DISABLE, DICS_ENABLE,
    DICS_FLAG_CONFIGSPECIFIC, DICS_FLAG_GLOBAL, DIF_PROPERTYCHANGE, DIGCF_ALLCLASSES,
    DIGCF_PRESENT, DN_HAS_PROBLEM, HDEVINFO, MAX_DEVICE_ID_LEN, SETUP_DI_PROPERTY_CHANGE_SCOPE,
    SP_CLASSINSTALL_HEADER, SP_DEVINFO_DATA, SP_DEVINSTALL_PARAMS_W, SP_PROPCHANGE_PARAMS,
    SetupDiCallClassInstaller, SetupDiCreateDeviceInfoList, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstallParamsW,
    SetupDiOpenDeviceInfoW, SetupDiSetClassInstallParamsW,
};
use windows::Win32::Foundation::{ERROR_NO_MORE_ITEMS, WIN32_ERROR};
use windows::core::PCWSTR;

use crate::status::{DeviceInfo, problem};
use crate::toggle::{ChangeOutcome, DeviceBackend, DeviceError};
use crate::win::wide;

/// The real device backend.
#[derive(Clone, Copy, Debug, Default)]
pub struct SetupApi;

impl DeviceBackend for SetupApi {
    fn find(&self, instance_id: &str) -> Result<Option<DeviceInfo>, DeviceError> {
        find_device(instance_id)
    }

    fn set_enabled(&self, instance_id: &str, enable: bool) -> Result<ChangeOutcome, DeviceError> {
        set_device_enabled(instance_id, enable)
    }
}

/// Every device currently present, of any class.
pub fn list_devices() -> Result<Vec<DeviceInfo>, DeviceError> {
    let set = DeviceInfoSet(
        unsafe {
            SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
        }
        .map_err(|err| win_error("Listing devices", &err))?,
    );

    let mut devices = Vec::new();
    for index in 0.. {
        let mut data = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if let Err(err) = unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
            if err.code() == ERROR_NO_MORE_ITEMS.to_hresult() {
                break;
            }
            return Err(win_error("Listing devices", &err));
        }
        // A device that vanishes mid-enumeration is skipped, not an error.
        if let Ok(info) = describe_devnode(data.DevInst, false) {
            devices.push(info);
        }
    }
    Ok(devices)
}

/// Look up one device. Instance IDs are case-insensitive. A device Windows
/// knows about but that isn't connected is returned with
/// [`problem::NOT_CONNECTED`].
pub fn find_device(instance_id: &str) -> Result<Option<DeviceInfo>, DeviceError> {
    let instance_id = instance_id.trim();
    if instance_id.is_empty() {
        return Ok(None);
    }

    for (flags, phantom) in [
        (CM_LOCATE_DEVNODE_NORMAL, false),
        (CM_LOCATE_DEVNODE_PHANTOM, true),
    ] {
        match locate(instance_id, flags) {
            Ok(devinst) => return describe_devnode(devinst, phantom).map(Some),
            Err(cr) if cr == CR_NO_SUCH_DEVNODE => continue,
            Err(cr) => return Err(cr_error(&format!("Looking up {instance_id}"), cr)),
        }
    }
    Ok(None)
}

/// Enable or disable a device the way Device Manager and `devcon` do
/// (`DIF_PROPERTYCHANGE`), so the setting persists across restarts.
pub fn set_device_enabled(instance_id: &str, enable: bool) -> Result<ChangeOutcome, DeviceError> {
    let verb = if enable { "Enabling" } else { "Disabling" };
    let context = format!("{verb} {}", instance_id.trim());
    let fail = |err: windows::core::Error| win_error(&context, &err);

    let set = DeviceInfoSet(unsafe { SetupDiCreateDeviceInfoList(None, None) }.map_err(fail)?);
    let mut data = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    let id = wide(instance_id.trim());
    unsafe { SetupDiOpenDeviceInfoW(set.0, PCWSTR(id.as_ptr()), None, 0, Some(&mut data)) }
        .map_err(fail)?;

    // devcon enables globally and for the current hardware profile, and
    // disables for the current profile.
    let scopes: &[SETUP_DI_PROPERTY_CHANGE_SCOPE] = if enable {
        &[DICS_FLAG_GLOBAL, DICS_FLAG_CONFIGSPECIFIC]
    } else {
        &[DICS_FLAG_CONFIGSPECIFIC]
    };
    for &scope in scopes {
        let params = SP_PROPCHANGE_PARAMS {
            ClassInstallHeader: SP_CLASSINSTALL_HEADER {
                cbSize: size_of::<SP_CLASSINSTALL_HEADER>() as u32,
                InstallFunction: DIF_PROPERTYCHANGE,
            },
            StateChange: if enable { DICS_ENABLE } else { DICS_DISABLE },
            Scope: scope,
            HwProfile: 0,
        };
        unsafe {
            SetupDiSetClassInstallParamsW(
                set.0,
                Some(&data),
                Some(&params.ClassInstallHeader),
                size_of::<SP_PROPCHANGE_PARAMS>() as u32,
            )
        }
        .map_err(fail)?;
        unsafe { SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, set.0, Some(&data)) }
            .map_err(fail)?;
    }

    let mut install = SP_DEVINSTALL_PARAMS_W {
        cbSize: size_of::<SP_DEVINSTALL_PARAMS_W>() as u32,
        ..Default::default()
    };
    let restart_required =
        unsafe { SetupDiGetDeviceInstallParamsW(set.0, Some(&data), &mut install) }.is_ok()
            && (install.Flags & (DI_NEEDREBOOT | DI_NEEDRESTART)).0 != 0;
    Ok(ChangeOutcome { restart_required })
}

/// Closes a SetupAPI device information set on drop.
struct DeviceInfoSet(HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

fn locate(instance_id: &str, flags: CM_LOCATE_DEVNODE_FLAGS) -> Result<u32, CONFIGRET> {
    let id = wide(instance_id);
    let mut devinst = 0;
    let cr = unsafe { CM_Locate_DevNodeW(&mut devinst, PCWSTR(id.as_ptr()), flags) };
    if cr == CR_SUCCESS {
        Ok(devinst)
    } else {
        Err(cr)
    }
}

fn describe_devnode(devinst: u32, phantom: bool) -> Result<DeviceInfo, DeviceError> {
    let mut id = [0u16; MAX_DEVICE_ID_LEN as usize + 1];
    let cr = unsafe { CM_Get_Device_IDW(devinst, &mut id, 0) };
    if cr != CR_SUCCESS {
        return Err(cr_error("Reading device ID", cr));
    }
    let instance_id = from_wide(&id);

    let problem = if phantom {
        problem::NOT_CONNECTED
    } else {
        let mut status = CM_DEVNODE_STATUS_FLAGS(0);
        let mut code = CM_PROB(0);
        let cr = unsafe { CM_Get_DevNode_Status(&mut status, &mut code, devinst, 0) };
        if cr != CR_SUCCESS {
            return Err(cr_error(&format!("Reading status of {instance_id}"), cr));
        }
        if status.contains(DN_HAS_PROBLEM) {
            code.0
        } else {
            problem::NONE
        }
    };

    // Prefer the friendly name Device Manager shows, if the device has one.
    let mut description = string_property(devinst, CM_DRP_FRIENDLYNAME);
    if description.is_empty() {
        description = string_property(devinst, CM_DRP_DEVICEDESC);
    }

    Ok(DeviceInfo {
        instance_id,
        description,
        manufacturer: string_property(devinst, CM_DRP_MFG),
        class: string_property(devinst, CM_DRP_CLASS),
        problem,
    })
}

/// A string registry property of a devnode, or empty if it has none.
fn string_property(devinst: u32, property: u32) -> String {
    let mut buffer = vec![0u16; 256];
    loop {
        let mut bytes = (buffer.len() * 2) as u32;
        let cr = unsafe {
            CM_Get_DevNode_Registry_PropertyW(
                devinst,
                property,
                None,
                Some(buffer.as_mut_ptr().cast()),
                &mut bytes,
                0,
            )
        };
        match cr {
            CR_SUCCESS => return from_wide(&buffer[..(bytes as usize / 2).min(buffer.len())]),
            CR_BUFFER_SMALL => buffer.resize((bytes as usize).div_ceil(2) + 1, 0),
            _ => return String::new(),
        }
    }
}

fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

fn win_error(context: &str, err: &windows::core::Error) -> DeviceError {
    DeviceError::new(format!(
        "{context}: {} (0x{:08X})",
        err.message(),
        err.code().0
    ))
}

fn cr_error(context: &str, cr: CONFIGRET) -> DeviceError {
    let code = unsafe { CM_MapCrToWin32Err(cr, 0) };
    if code == 0 {
        return DeviceError::new(format!("{context}: Configuration Manager error {}", cr.0));
    }
    let err = windows::core::Error::from(WIN32_ERROR(code).to_hresult());
    DeviceError::new(format!("{context}: {} (CONFIGRET {})", err.message(), cr.0))
}
