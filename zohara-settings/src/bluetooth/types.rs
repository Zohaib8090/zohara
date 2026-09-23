// bluetooth/types.rs — Data model and inter-thread messaging types.

use std::collections::HashMap;
use zvariant::OwnedObjectPath;

/// A complete, structured snapshot of a single Bluetooth device.
/// Every field is sourced from org.bluez.Device1 or org.bluez.Battery1.
#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    /// D-Bus object path — the stable key for this device.
    pub object_path: OwnedObjectPath,
    /// Hardware MAC address, always present.
    pub address: String,
    /// org.bluez.Device1.Name — may be absent for devices that haven't advertised it.
    pub name: Option<String>,
    /// org.bluez.Device1.Alias — always present (BlueZ defaults to the MAC address).
    pub alias: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    /// org.bluez.Device1.Icon — e.g. "audio-headset", useful for UI glyphs.
    pub icon: Option<String>,
    /// org.bluez.Battery1.Percentage — only present if BlueZ exposes the interface.
    pub battery_percent: Option<u8>,
    /// org.bluez.Device1.Class — device-type bitfield, fallback classifier if Icon is absent.
    pub class: Option<u32>,
}

impl BluetoothDevice {
    /// Returns the best human-readable name for display in the UI.
    /// Priority: Name > Alias (which always falls back to MAC in BlueZ).
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.alias)
    }
}

/// Shared, cache-wide state snapshot.
/// The worker thread is the exclusive writer; GTK reads via Arc<ArcSwap<DeviceCache>>.
#[derive(Debug, Clone, Default)]
pub struct DeviceCache {
    pub devices: HashMap<OwnedObjectPath, BluetoothDevice>,
    /// True when the system D-Bus connection to org.bluez is live.
    pub bluez_available: bool,
    /// Reflects org.bluez.Adapter1.Powered on the default adapter.
    pub adapter_powered: bool,
}

/// Commands the GTK4 UI thread sends to the Bluetooth worker thread.
#[derive(Debug)]
pub enum Command {
    Connect(OwnedObjectPath),
    Disconnect(OwnedObjectPath),
    Trust(OwnedObjectPath, bool),
    Remove(OwnedObjectPath),
    StartDiscovery,
    StopDiscovery,
    /// Orderly shutdown of the worker task.
    Shutdown,
}

/// Lightweight "cache changed" notification sent from worker → GTK.
/// The UI doesn't care *what* changed — it re-reads the full cache once.
#[derive(Debug)]
pub struct CacheDirty;
