// D-Bus helpers for NetworkManager, BlueZ, UPower via zbus
// Each function returns a glib Future compatible with glib::spawn_future_local

use zbus::Connection;
use std::time::Duration;
use tokio::time::timeout;

const DBUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Get the current Wi-Fi enabled state from NetworkManager via D-Bus with 5s timeout
pub async fn nm_wifi_enabled() -> Result<bool, String> {
    let fut = async {
        let conn = Connection::system().await.map_err(|e| e.to_string())?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await
        .map_err(|e| e.to_string())?;

        let enabled: bool = proxy.get_property("WirelessEnabled").await.map_err(|e| e.to_string())?;
        Ok(enabled)
    };

    match timeout(DBUS_TIMEOUT, fut).await {
        Ok(res) => res,
        Err(_) => Err("D-Bus request timed out".to_string()),
    }
}

/// Set Wi-Fi enabled state with 5s timeout
pub async fn nm_set_wifi(enabled: bool) -> Result<(), String> {
    let fut = async {
        let conn = Connection::system().await.map_err(|e| e.to_string())?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await
        .map_err(|e| e.to_string())?;

        proxy.set_property("WirelessEnabled", enabled).await.map_err(|e| e.to_string())?;
        Ok(())
    };

    match timeout(DBUS_TIMEOUT, fut).await {
        Ok(res) => res,
        Err(_) => Err("D-Bus request timed out".to_string()),
    }
}

/// Get battery percentage from UPower
pub async fn upower_battery() -> Result<(f64, bool), zbus::Error> {
    let conn = Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT0",
        "org.freedesktop.UPower.Device",
    )
    .await?;
    let percentage: f64 = proxy.get_property("Percentage").await.unwrap_or(0.0);
    let state: u32 = proxy.get_property("State").await.unwrap_or(0);
    // State 1 = Charging, 2 = Discharging
    Ok((percentage, state == 1))
}

/// Check if Bluetooth adapter is powered
pub async fn bluez_adapter_powered() -> Result<bool, zbus::Error> {
    let conn = Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.bluez",
        "/org/bluez/hci0",
        "org.bluez.Adapter1",
    )
    .await?;
    let powered: bool = proxy.get_property("Powered").await?;
    Ok(powered)
}

/// Set Bluetooth adapter power
pub async fn bluez_set_adapter_power(on: bool) -> Result<(), zbus::Error> {
    let conn = Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.bluez",
        "/org/bluez/hci0",
        "org.bluez.Adapter1",
    )
    .await?;
    proxy.set_property("Powered", on).await?;
    Ok(())
}
