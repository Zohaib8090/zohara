// bluetooth/connection.rs — D-Bus connection establishment with exponential backoff.

use std::time::Duration;
use zbus::Connection;

/// Maximum backoff delay before we give up retrying for this cycle.
const MAX_BACKOFF_SECS: u64 = 30;

/// Attempt to connect to the system D-Bus, retrying with capped exponential backoff
/// if the connection or service is unavailable.
///
/// Returns `Some(Connection)` on success or `None` if the loop is cancelled via
/// the provided shutdown signal.
pub async fn connect_with_backoff(
    shutdown: &tokio::sync::CancellationToken,
) -> Option<Connection> {
    let mut delay_secs: u64 = 1;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                return None;
            }
            result = Connection::system() => {
                match result {
                    Ok(conn) => {
                        // Verify org.bluez is actually running before returning the connection
                        match verify_bluez(&conn).await {
                            true => {
                                tracing::info!("Connected to system D-Bus; org.bluez is live.");
                                return Some(conn);
                            }
                            false => {
                                tracing::warn!(
                                    "D-Bus connected but org.bluez not available. \
                                     Retrying in {delay_secs}s."
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("System D-Bus connection failed: {e}. Retrying in {delay_secs}s.");
                    }
                }
            }
        }

        tokio::select! {
            _ = shutdown.cancelled() => return None,
            _ = tokio::time::sleep(Duration::from_secs(delay_secs)) => {}
        }

        // Cap the backoff so we don't wait forever.
        delay_secs = (delay_secs * 2).min(MAX_BACKOFF_SECS);
    }
}

/// Pings the org.bluez well-known name to confirm the daemon is running.
async fn verify_bluez(conn: &Connection) -> bool {
    conn.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "GetNameOwner",
        &("org.bluez",),
    )
    .await
    .is_ok()
}
