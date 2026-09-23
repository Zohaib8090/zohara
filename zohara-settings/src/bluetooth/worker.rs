// bluetooth/worker.rs — The async task that permanently owns the D-Bus connection.
//
// Responsibilities:
//   1. Seed the DeviceCache from GetManagedObjects on startup.
//   2. Subscribe to InterfacesAdded / InterfacesRemoved to track device lifetime.
//   3. Subscribe to PropertiesChanged on Device1 and Battery1 to track state.
//   4. Execute Commands from the GTK thread (connect, disconnect, trust, etc.).
//   5. Rebuild state from scratch after a D-Bus disconnection (backoff in connection.rs).

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::CancellationToken;
use zbus::fdo::{ObjectManagerProxy, PropertiesProxy};
use zbus::Connection;
use zvariant::{OwnedObjectPath, OwnedValue, Value};

use super::connection::connect_with_backoff;
use super::types::{BluetoothDevice, CacheDirty, Command, DeviceCache};

const DEVICE1_IFACE: &str = "org.bluez.Device1";
const BATTERY1_IFACE: &str = "org.bluez.Battery1";
const ADAPTER1_IFACE: &str = "org.bluez.Adapter1";

pub async fn run(
    cache: Arc<ArcSwap<DeviceCache>>,
    dirty_tx: UnboundedSender<CacheDirty>,
    mut cmd_rx: UnboundedReceiver<Command>,
    shutdown: CancellationToken,
) {
    loop {
        // --- Phase A: establish connection ---
        let conn = match connect_with_backoff(&shutdown).await {
            Some(c) => c,
            None => return, // shutdown was requested during backoff
        };

        // --- Phase B: seed cache ---
        if let Err(e) = seed_cache(&conn, &cache, &dirty_tx).await {
            tracing::error!("Failed to seed device cache: {e}. Reconnecting.");
            continue;
        }

        // --- Phase C: run event loop ---
        let disconnect = run_event_loop(&conn, &cache, &dirty_tx, &mut cmd_rx, &shutdown).await;

        if disconnect {
            // D-Bus dropped mid-session; mark unavailable and loop back to reconnect.
            update_cache(&cache, &dirty_tx, |c| {
                c.bluez_available = false;
                c.devices.clear();
            });
        } else {
            // Clean shutdown.
            return;
        }
    }
}

/// Query the entire BlueZ object tree and build the initial DeviceCache.
async fn seed_cache(
    conn: &Connection,
    cache: &Arc<ArcSwap<DeviceCache>>,
    dirty_tx: &UnboundedSender<CacheDirty>,
) -> zbus::Result<()> {
    let proxy = ObjectManagerProxy::builder(conn)
        .destination("org.bluez")?
        .path("/")?
        .build()
        .await?;

    let objects = proxy.get_managed_objects().await?;

    let mut new_cache = DeviceCache {
        bluez_available: true,
        adapter_powered: false,
        devices: HashMap::new(),
    };

    for (path, interfaces) in &objects {
        // Check adapter power state
        if let Some(adapter_props) = interfaces.get(ADAPTER1_IFACE) {
            if let Some(powered) = extract_bool(adapter_props, "Powered") {
                new_cache.adapter_powered = powered;
            }
        }

        // Parse devices
        if let Some(device_props) = interfaces.get(DEVICE1_IFACE) {
            if let Some(device) =
                parse_device(path.clone(), device_props, interfaces.get(BATTERY1_IFACE))
            {
                new_cache.devices.insert(path.clone(), device);
            }
        }
    }

    cache.store(Arc::new(new_cache));
    let _ = dirty_tx.send(CacheDirty);
    Ok(())
}

/// Main event loop. Returns `true` if the loop exited due to a D-Bus error (reconnect needed),
/// or `false` if a clean `Shutdown` command was received.
async fn run_event_loop(
    conn: &Connection,
    cache: &Arc<ArcSwap<DeviceCache>>,
    dirty_tx: &UnboundedSender<CacheDirty>,
    cmd_rx: &mut UnboundedReceiver<Command>,
    shutdown: &CancellationToken,
) -> bool {
    // Subscribe to InterfacesAdded / InterfacesRemoved
    let obj_proxy = match ObjectManagerProxy::builder(conn)
        .destination("org.bluez")
        .and_then(|b| b.path("/"))
        .unwrap()
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to create ObjectManagerProxy: {e}");
            return true; // reconnect
        }
    };

    let mut interfaces_added = match obj_proxy.receive_interfaces_added().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to subscribe InterfacesAdded: {e}");
            return true;
        }
    };

    let mut interfaces_removed = match obj_proxy.receive_interfaces_removed().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to subscribe InterfacesRemoved: {e}");
            return true;
        }
    };

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => return false,

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Command::Shutdown => return false,
                    Command::Connect(path) => {
                        execute_device_method(conn, &path, "Connect").await;
                    }
                    Command::Disconnect(path) => {
                        execute_device_method(conn, &path, "Disconnect").await;
                    }
                    Command::Trust(path, trusted) => {
                        set_device_property(conn, &path, "Trusted", Value::Bool(trusted)).await;
                    }
                    Command::Remove(path) => {
                        // Remove via adapter's RemoveDevice method
                        if let Err(e) = conn.call_method(
                            Some("org.bluez"),
                            "/org/bluez/hci0",
                            Some("org.bluez.Adapter1"),
                            "RemoveDevice",
                            &(&path,),
                        ).await {
                            tracing::warn!("RemoveDevice failed for {path}: {e}");
                        }
                    }
                    Command::StartDiscovery => {
                        let _ = conn.call_method(
                            Some("org.bluez"),
                            "/org/bluez/hci0",
                            Some("org.bluez.Adapter1"),
                            "StartDiscovery",
                            &(),
                        ).await;
                    }
                    Command::StopDiscovery => {
                        let _ = conn.call_method(
                            Some("org.bluez"),
                            "/org/bluez/hci0",
                            Some("org.bluez.Adapter1"),
                            "StopDiscovery",
                            &(),
                        ).await;
                    }
                }
            }

            Some(signal) = interfaces_added.next() => {
                if let Ok(args) = signal.args() {
                    let path = args.object_path.clone();
                    let interfaces: HashMap<String, HashMap<String, OwnedValue>> = args
                        .interfaces_and_properties
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect();

                    if let Some(device_props) = interfaces.get(DEVICE1_IFACE) {
                        if let Some(device) = parse_device(
                            path.clone(),
                            device_props,
                            interfaces.get(BATTERY1_IFACE),
                        ) {
                            update_cache(cache, dirty_tx, |c| {
                                c.devices.insert(path.clone(), device);
                            });
                        }
                    }
                }
            }

            Some(signal) = interfaces_removed.next() => {
                if let Ok(args) = signal.args() {
                    let path = args.object_path.clone();
                    if args.interfaces.contains(&DEVICE1_IFACE) {
                        update_cache(cache, dirty_tx, |c| {
                            c.devices.remove(&path);
                        });
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn update_cache(
    cache: &Arc<ArcSwap<DeviceCache>>,
    dirty_tx: &UnboundedSender<CacheDirty>,
    f: impl FnOnce(&mut DeviceCache),
) {
    let mut new_cache = (**cache.load()).clone();
    f(&mut new_cache);
    cache.store(Arc::new(new_cache));
    let _ = dirty_tx.send(CacheDirty);
}

fn parse_device(
    path: OwnedObjectPath,
    props: &HashMap<String, OwnedValue>,
    battery_props: Option<&HashMap<String, OwnedValue>>,
) -> Option<BluetoothDevice> {
    let address = extract_str(props, "Address")?.to_string();
    let alias = extract_str(props, "Alias")
        .unwrap_or(&address)
        .to_string();
    let name = extract_str(props, "Name").map(|s| s.to_string());
    let paired = extract_bool(props, "Paired").unwrap_or(false);
    let connected = extract_bool(props, "Connected").unwrap_or(false);
    let trusted = extract_bool(props, "Trusted").unwrap_or(false);
    let icon = extract_str(props, "Icon").map(|s| s.to_string());
    let class = extract_u32(props, "Class");

    let battery_percent = battery_props.and_then(|bp| {
        bp.get("Percentage").and_then(|v| {
            if let Ok(b) = u8::try_from(v) {
                Some(b)
            } else {
                None
            }
        })
    });

    Some(BluetoothDevice {
        object_path: path,
        address,
        alias,
        name,
        paired,
        connected,
        trusted,
        icon,
        class,
        battery_percent,
    })
}

fn extract_str<'a>(props: &'a HashMap<String, OwnedValue>, key: &str) -> Option<&'a str> {
    props.get(key).and_then(|v| v.downcast_ref::<str>())
}

fn extract_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    props.get(key).and_then(|v| bool::try_from(v).ok())
}

fn extract_u32(props: &HashMap<String, OwnedValue>, key: &str) -> Option<u32> {
    props.get(key).and_then(|v| u32::try_from(v).ok())
}

async fn execute_device_method(conn: &Connection, path: &OwnedObjectPath, method: &str) {
    if let Err(e) = conn
        .call_method(
            Some("org.bluez"),
            path.as_str(),
            Some(DEVICE1_IFACE),
            method,
            &(),
        )
        .await
    {
        tracing::warn!("Device1.{method} on {path} failed: {e}");
    }
}

async fn set_device_property(
    conn: &Connection,
    path: &OwnedObjectPath,
    property: &str,
    value: Value<'_>,
) {
    let proxy = match PropertiesProxy::builder(conn)
        .destination("org.bluez")
        .and_then(|b| b.path(path.as_str()))
        .unwrap()
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Could not build PropertiesProxy for {path}: {e}");
            return;
        }
    };

    if let Err(e) = proxy.set(DEVICE1_IFACE, property, &value).await {
        tracing::warn!("Set {property} on {path} failed: {e}");
    }
}
