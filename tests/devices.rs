//! Read-only checks against the real Configuration Manager on the machine
//! running the tests (works unelevated, including on CI runners). Nothing
//! here ever enables or disables a device.

use donttouchmebro::device::{SetupApi, find_device, list_devices};
use donttouchmebro::status::DeviceState;
use donttouchmebro::toggle::{DeviceBackend, query_state};

#[test]
fn lists_present_devices_with_ids_and_classes() {
    let devices = list_devices().expect("list devices");
    assert!(
        !devices.is_empty(),
        "every Windows machine has some devices"
    );
    assert!(devices.iter().all(|d| !d.instance_id.is_empty()));
    assert!(devices.iter().any(|d| !d.class.is_empty()));
    assert!(devices.iter().any(|d| !d.description.is_empty()));
}

#[test]
fn finds_a_listed_device_by_id_case_insensitively() {
    let devices = list_devices().expect("list devices");
    let device = &devices[0];

    let found = find_device(&device.instance_id)
        .expect("find")
        .expect("device exists");
    assert_eq!(found.instance_id, device.instance_id);
    assert_eq!(found.class, device.class);

    let lower = format!("  {}  ", device.instance_id.to_lowercase());
    let found = find_device(&lower)
        .expect("find")
        .expect("lookup ignores case");
    assert_eq!(found.instance_id, device.instance_id);
}

#[test]
fn unknown_or_empty_ids_are_not_found() {
    assert_eq!(find_device(r"DTMB\DOES-NOT-EXIST\0").expect("find"), None);
    assert_eq!(find_device("").expect("find"), None);
    assert_eq!(find_device("   ").expect("find"), None);
}

#[test]
fn backend_maps_states_for_the_tray() {
    let backend = SetupApi;
    let device = list_devices().expect("list devices").remove(0);

    assert_eq!(
        query_state(&backend, None).unwrap(),
        DeviceState::NotConfigured
    );
    assert_eq!(
        query_state(&backend, Some(r"DTMB\DOES-NOT-EXIST\0")).unwrap(),
        DeviceState::NotFound
    );
    assert!(matches!(
        query_state(&backend, Some(&device.instance_id)).unwrap(),
        DeviceState::Device(info) if info.instance_id == device.instance_id
    ));
    assert!(backend.find(&device.instance_id).unwrap().is_some());
}
