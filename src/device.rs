//! Device access through the Configuration Manager (cfgmgr32) and SetupAPI —
//! the same APIs Device Manager and `pnputil` use, without WMI/COM.
//!
//! Reading device state works unelevated; enabling or disabling needs
//! Administrator (the app manifest requests it).

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_DEVNODE_STATUS_FLAGS, CM_DISABLE_PERSIST, CM_DISABLE_UI_NOT_OK, CM_DRP_CLASS,
    CM_DRP_DEVICEDESC, CM_DRP_FRIENDLYNAME, CM_DRP_MFG, CM_Disable_DevNode, CM_Enable_DevNode,
    CM_Get_DevNode_PropertyW, CM_Get_DevNode_Registry_PropertyW, CM_Get_DevNode_Status,
    CM_Get_Device_IDW, CM_Get_Parent, CM_LOCATE_DEVNODE_FLAGS, CM_LOCATE_DEVNODE_NORMAL,
    CM_LOCATE_DEVNODE_PHANTOM, CM_Locate_DevNodeW, CM_MapCrToWin32Err, CM_PROB, CONFIGRET,
    CR_BUFFER_SMALL, CR_NO_SUCH_DEVNODE, CR_NOT_DISABLEABLE, CR_REMOVE_VETOED, CR_SUCCESS,
    DI_NEEDREBOOT, DI_NEEDRESTART, DICS_DISABLE, DICS_ENABLE, DICS_FLAG_CONFIGSPECIFIC,
    DICS_FLAG_GLOBAL, DIF_PROPERTYCHANGE, DIGCF_ALLCLASSES, DIGCF_PRESENT, DN_DISABLEABLE,
    DN_HAS_PROBLEM, HDEVINFO, MAX_DEVICE_ID_LEN, SETUP_DI_PROPERTY_CHANGE_SCOPE,
    SP_CLASSINSTALL_HEADER, SP_DEVINFO_DATA, SP_DEVINSTALL_PARAMS_W, SP_PROPCHANGE_PARAMS,
    SetupDiCallClassInstaller, SetupDiCreateDeviceInfoList, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstallParamsW,
    SetupDiOpenDeviceInfoW, SetupDiSetClassInstallParamsW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_ContainerId, DEVPROP_TYPE_GUID,
    DEVPROP_TYPE_STRING, DEVPROPTYPE,
};
use windows::Win32::Foundation::{
    DEVPROPKEY, ERROR_IN_WOW64, ERROR_NO_MORE_ITEMS, ERROR_NO_SUCH_DEVINST, ERROR_NOT_DISABLEABLE,
    WIN32_ERROR,
};
use windows::core::{GUID, PCWSTR};

use crate::naming::{self, NodeNames};
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

/// Enable or disable a device the way `pnputil /enable-device` and
/// `/disable-device` do (`CM_Enable_DevNode` / `CM_Disable_DevNode`, with
/// the disable persisting across restarts).
///
/// SetupAPI's `DIF_PROPERTYCHANGE` (what `devcon` uses) was tried first, but
/// for some devices, such as Bluetooth LE ones, it reports success without
/// changing anything. It's kept as a fallback for enabling a device that
/// `CM_Enable_DevNode` leaves disabled, e.g. one disabled that way or in
/// Device Manager.
pub fn set_device_enabled(instance_id: &str, enable: bool) -> Result<ChangeOutcome, DeviceError> {
    let instance_id = instance_id.trim();
    let verb = if enable { "Enabling" } else { "Disabling" };
    let context = format!("{verb} {instance_id}");

    let devinst = match locate(instance_id, CM_LOCATE_DEVNODE_NORMAL) {
        Ok(devinst) => devinst,
        Err(cr) if cr == CR_NO_SUCH_DEVNODE => {
            return Err(DeviceError::with_context(
                context,
                "The device isn't connected.",
            ));
        }
        Err(cr) => return Err(cr_error(&context, cr)),
    };

    let cr = unsafe {
        if enable {
            CM_Enable_DevNode(devinst, 0)
        } else {
            CM_Disable_DevNode(devinst, CM_DISABLE_UI_NOT_OK | CM_DISABLE_PERSIST)
        }
    };
    match cr {
        CR_SUCCESS => {}
        CR_REMOVE_VETOED => {
            return Err(DeviceError::with_context(
                context,
                "Something is using the device, so Windows wouldn't stop it. \
                 Close whatever is using it (or restart) and try again.",
            ));
        }
        CR_NOT_DISABLEABLE => {
            return Err(DeviceError::with_context(
                context,
                "Windows doesn't allow this device to be disabled.",
            ));
        }
        cr => return Err(cr_error(&context, cr)),
    }

    let (problem, _) = devnode_status(devinst).unwrap_or((problem::NONE, false));
    if enable && problem == problem::DISABLED {
        return set_device_enabled_setupapi(instance_id, true);
    }
    Ok(ChangeOutcome {
        restart_required: problem == problem::RESTART_REQUIRED,
    })
}

/// Problem code and `DN_DISABLEABLE` for a present devnode.
fn devnode_status(devinst: u32) -> Result<(u32, bool), CONFIGRET> {
    let mut status = CM_DEVNODE_STATUS_FLAGS(0);
    let mut code = CM_PROB(0);
    let cr = unsafe { CM_Get_DevNode_Status(&mut status, &mut code, devinst, 0) };
    if cr != CR_SUCCESS {
        return Err(cr);
    }
    let problem = if status.contains(DN_HAS_PROBLEM) {
        code.0
    } else {
        problem::NONE
    };
    Ok((problem, status.contains(DN_DISABLEABLE)))
}

/// Enable or disable through SetupAPI (`DIF_PROPERTYCHANGE`), as `devcon`
/// does; see [`set_device_enabled`] for when this is used.
fn set_device_enabled_setupapi(
    instance_id: &str,
    enable: bool,
) -> Result<ChangeOutcome, DeviceError> {
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

    let (problem, can_disable) = if phantom {
        (problem::NOT_CONNECTED, false)
    } else {
        devnode_status(devinst)
            .map_err(|cr| cr_error(&format!("Reading status of {instance_id}"), cr))?
    };

    let kind = string_property(devinst, CM_DRP_DEVICEDESC);
    let (chain, has_container) = ancestry(devinst);
    let product_name = naming::product_name(&chain, has_container).unwrap_or_default();

    Ok(DeviceInfo {
        instance_id,
        // The name Device Manager shows.
        description: node_name(devinst),
        product_name,
        kind,
        manufacturer: string_property(devinst, CM_DRP_MFG),
        class: string_property(devinst, CM_DRP_CLASS),
        problem,
        can_disable,
    })
}

/// Friendly name, or the driver's description if there isn't one.
fn node_name(devinst: u32) -> String {
    let name = string_property(devinst, CM_DRP_FRIENDLYNAME);
    if name.is_empty() {
        string_property(devinst, CM_DRP_DEVICEDESC)
    } else {
        name
    }
}

/// Device-tree nodes that are system buses rather than parts of a product;
/// the walk up from a device stops before reaching them.
const SYSTEM_ENUMERATORS: [&str; 6] = [
    "PCI\\",
    "ACPI\\",
    "ACPI_HAL\\",
    "HTREE\\",
    "UEFI\\",
    "SWD\\",
];

/// "No container": internal and system devices share this container ID.
const NO_CONTAINER: GUID = GUID::from_u128(0x00000000_0000_0000_ffff_ffffffffffff);

/// The names of a device and its ancestors, for [`naming::product_name`],
/// and whether the device has a real device container.
///
/// The walk stays inside the device's container (one physical product), or
/// for devices without one, goes up a few levels; either way it stops
/// before system buses such as PCI or ACPI.
fn ancestry(devinst: u32) -> (Vec<NodeNames>, bool) {
    let container =
        guid_property(devinst, &DEVPKEY_Device_ContainerId).filter(|c| *c != NO_CONTAINER);
    let max_depth = if container.is_some() { 8 } else { 3 };

    let mut chain = Vec::new();
    let mut node = devinst;
    for depth in 0..=max_depth {
        chain.push(NodeNames {
            name: node_name(node),
            bus_name: string_devprop(node, &DEVPKEY_Device_BusReportedDeviceDesc),
        });
        if depth == max_depth {
            break;
        }

        let mut parent = 0;
        if unsafe { CM_Get_Parent(&mut parent, node, 0) } != CR_SUCCESS {
            break;
        }
        let parent_id = device_id(parent).to_uppercase();
        if parent_id.is_empty()
            || SYSTEM_ENUMERATORS
                .iter()
                .any(|prefix| parent_id.starts_with(prefix))
        {
            break;
        }
        if container.is_some() && guid_property(parent, &DEVPKEY_Device_ContainerId) != container {
            break;
        }
        node = parent;
    }
    (chain, container.is_some())
}

fn device_id(devinst: u32) -> String {
    let mut id = [0u16; MAX_DEVICE_ID_LEN as usize + 1];
    if unsafe { CM_Get_Device_IDW(devinst, &mut id, 0) } == CR_SUCCESS {
        from_wide(&id)
    } else {
        String::new()
    }
}

/// A string device property (`DEVPKEY_*`), or empty if it isn't set.
fn string_devprop(devinst: u32, key: &DEVPROPKEY) -> String {
    let mut buffer = vec![0u16; 128];
    loop {
        let mut kind = DEVPROPTYPE(0);
        let mut bytes = (buffer.len() * 2) as u32;
        let cr = unsafe {
            CM_Get_DevNode_PropertyW(
                devinst,
                key,
                &mut kind,
                Some(buffer.as_mut_ptr().cast()),
                &mut bytes,
                0,
            )
        };
        match cr {
            CR_SUCCESS if kind == DEVPROP_TYPE_STRING => {
                return from_wide(&buffer[..(bytes as usize / 2).min(buffer.len())]);
            }
            CR_BUFFER_SMALL => buffer.resize((bytes as usize).div_ceil(2) + 1, 0),
            _ => return String::new(),
        }
    }
}

/// A GUID device property, or `None` if it isn't set.
fn guid_property(devinst: u32, key: &DEVPROPKEY) -> Option<GUID> {
    let mut guid = GUID::zeroed();
    let mut kind = DEVPROPTYPE(0);
    let mut bytes = size_of::<GUID>() as u32;
    let cr = unsafe {
        CM_Get_DevNode_PropertyW(
            devinst,
            key,
            &mut kind,
            Some((&mut guid as *mut GUID).cast()),
            &mut bytes,
            0,
        )
    };
    (cr == CR_SUCCESS && kind == DEVPROP_TYPE_GUID).then_some(guid)
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
    DeviceError::with_context(context, describe_error(err))
}

/// A readable message for a Windows error. SetupAPI's own errors
/// (0xE000xxxx) aren't in the system message table, so `message()` is empty
/// for them; spell out the ones users can actually hit.
fn describe_error(err: &windows::core::Error) -> String {
    let code = err.code().0 as u32;
    let known = [
        (
            ERROR_NOT_DISABLEABLE,
            "Windows doesn't allow this device to be disabled.",
        ),
        (ERROR_NO_SUCH_DEVINST, "The device no longer exists."),
        (ERROR_IN_WOW64, "This needs the 64-bit version of the app."),
    ];
    if let Some((_, text)) = known.iter().find(|(known, _)| known.0 == code) {
        return format!("{text} (0x{code:08X})");
    }
    match err.message().trim() {
        "" => format!("Windows error 0x{code:08X}"),
        message => format!("{message} (0x{code:08X})"),
    }
}

fn cr_error(context: &str, cr: CONFIGRET) -> DeviceError {
    let code = unsafe { CM_MapCrToWin32Err(cr, 0) };
    if code == 0 {
        return DeviceError::with_context(context, format!("Configuration Manager error {}", cr.0));
    }
    let err = windows::core::Error::from(WIN32_ERROR(code).to_hresult());
    DeviceError::with_context(
        context,
        format!("{} (CONFIGRET {})", describe_error(&err), cr.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(code: WIN32_ERROR) -> windows::core::Error {
        windows::core::Error::from(code.to_hresult())
    }

    #[test]
    fn setupapi_errors_get_readable_messages() {
        // These have no system message text, so this used to log ": (0xE0000231)".
        assert_eq!(
            describe_error(&error(ERROR_NOT_DISABLEABLE)),
            "Windows doesn't allow this device to be disabled. (0xE0000231)"
        );
        assert!(
            describe_error(&error(ERROR_NO_SUCH_DEVINST))
                .starts_with("The device no longer exists.")
        );
    }

    #[test]
    fn other_errors_use_the_system_message() {
        let text = describe_error(&error(windows::Win32::Foundation::ERROR_ACCESS_DENIED));
        assert!(text.ends_with("(0x80070005)"), "{text}");
        assert!(text.len() > "(0x80070005)".len(), "{text}");
    }

    #[test]
    fn device_error_keeps_context_for_the_log_only() {
        let err = win_error(
            r"Disabling ROOT\HIDCLASS\0000",
            &error(ERROR_NOT_DISABLEABLE),
        );
        assert_eq!(
            err.reason,
            "Windows doesn't allow this device to be disabled. (0xE0000231)"
        );
        assert_eq!(
            err.to_string(),
            r"Disabling ROOT\HIDCLASS\0000: Windows doesn't allow this device to be disabled. (0xE0000231)"
        );
    }
}
