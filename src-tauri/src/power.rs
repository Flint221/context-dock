//! Windows 11 energy saver and the older battery-saver flag are distinct.
use std::sync::{
    atomic::{AtomicU32, Ordering},
    OnceLock,
};
use windows_sys::{
    core::GUID,
    Win32::{
        System::Power::{
            GetSystemPowerStatus, PowerSettingRegisterNotification,
            DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, POWERBROADCAST_SETTING, SYSTEM_POWER_STATUS,
        },
        UI::WindowsAndMessaging::{DEVICE_NOTIFY_CALLBACK, PBT_POWERSETTINGCHANGE},
    },
};

// Documented GUID_ENERGY_SAVER_STATUS: OFF=0, STANDARD=1, HIGH_SAVINGS=2.
// It is not yet present in the SDK used by windows-sys 0.61.
const ENERGY: GUID = GUID::from_u128(0x550e8400_e29b_41d4_a716_446655440000);
static STATUS: AtomicU32 = AtomicU32::new(u32::MAX);
static REGISTRATION: OnceLock<Option<usize>> = OnceLock::new();

struct Subscription(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS);
// Immutable process-lifetime callback descriptor; its context is always null.
// The OS only reads it and the callback communicates through STATUS's atomic.
unsafe impl Sync for Subscription {}
static SUBSCRIPTION: Subscription = Subscription(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
    Callback: Some(changed),
    Context: std::ptr::null_mut(),
});

fn decode(kind: u32, guid: GUID, bytes: &[u8]) -> Option<u32> {
    if kind != PBT_POWERSETTINGCHANGE
        || guid.data1 != ENERGY.data1
        || guid.data2 != ENERGY.data2
        || guid.data3 != ENERGY.data3
        || guid.data4 != ENERGY.data4
    {
        return None;
    }
    let value = u32::from_le_bytes(bytes.try_into().ok()?);
    (value <= 2).then_some(value)
}

unsafe extern "system" fn changed(
    _: *const core::ffi::c_void,
    kind: u32,
    setting: *const core::ffi::c_void,
) -> u32 {
    if !setting.is_null() && kind == PBT_POWERSETTINGCHANGE {
        // Windows owns the notification buffer for this callback. Read only
        // the documented four-byte payload, never its flexible array length.
        let setting = unsafe { &*setting.cast::<POWERBROADCAST_SETTING>() };
        if setting.DataLength == 4 {
            let bytes = unsafe { std::slice::from_raw_parts(setting.Data.as_ptr(), 4) };
            if let Some(value) = decode(kind, setting.PowerSetting, bytes) {
                STATUS.store(value, Ordering::Release);
            }
        }
    }
    0
}

pub fn saving_energy() -> bool {
    REGISTRATION.get_or_init(|| {
        let mut handle = std::ptr::null_mut();
        // Registration also delivers the initial status. One subscription
        // lives for this process; Windows releases it at process termination.
        let result = unsafe {
            PowerSettingRegisterNotification(
                &ENERGY,
                DEVICE_NOTIFY_CALLBACK,
                (&SUBSCRIPTION.0 as *const DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS)
                    .cast_mut()
                    .cast(),
                &mut handle,
            )
        };
        (result == 0).then_some(handle as usize)
    });
    let mut legacy = SYSTEM_POWER_STATUS::default();
    let battery = unsafe { GetSystemPowerStatus(&mut legacy) != 0 && legacy.SystemStatusFlag == 1 };
    battery || matches!(STATUS.load(Ordering::Acquire), 1 | 2)
}

#[cfg(test)]
#[test]
fn energy_notifications_validate_guid_kind_size_and_known_modes() {
    for value in 0u32..=2 {
        assert_eq!(
            decode(PBT_POWERSETTINGCHANGE, ENERGY, &value.to_le_bytes()),
            Some(value)
        );
    }
    assert_eq!(decode(0, ENERGY, &1u32.to_le_bytes()), None);
    assert_eq!(
        decode(
            PBT_POWERSETTINGCHANGE,
            GUID::from_u128(0),
            &1u32.to_le_bytes()
        ),
        None
    );
    assert_eq!(decode(PBT_POWERSETTINGCHANGE, ENERGY, &[1]), None);
    assert_eq!(
        decode(PBT_POWERSETTINGCHANGE, ENERGY, &3u32.to_le_bytes()),
        None
    );
}

#[cfg(test)]
#[test]
#[ignore = "Explicit live Windows energy-saver check; set the OS mode and expected state first"]
fn windows_energy_saver_matches_selected_os_mode() {
    let expected: bool = std::env::var("CONTEXTDOCK_EXPECT_ENERGY_SAVER")
        .expect("Set the expected live OS state explicitly.")
        .parse()
        .unwrap();
    // Registration may deliver its initial notification asynchronously.
    for _ in 0..30 {
        let actual = saving_energy();
        if STATUS.load(Ordering::Acquire) != u32::MAX && actual == expected {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_ne!(STATUS.load(Ordering::Acquire), u32::MAX);
    assert_eq!(saving_energy(), expected);
}
