// bluetooth/manager.rs — Public handle for the GTK4 UI thread.
//
// This is the ONLY type GTK4 code imports. It never touches D-Bus directly:
//   - Read state via .cache()
//   - Send commands via .send_command()

use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::CancellationToken;

use super::types::{CacheDirty, Command, DeviceCache};
use super::worker;

pub struct BluetoothManager {
    cache: Arc<ArcSwap<DeviceCache>>,
    cmd_tx: UnboundedSender<Command>,
    shutdown: CancellationToken,
}

impl BluetoothManager {
    /// Start the background worker thread and return the manager handle.
    ///
    /// `dirty_callback` is called on the GTK main thread every time the cache
    /// changes. Use it to trigger a UI redraw (e.g. `widget.queue_draw()` or
    /// `glib::idle_add_once`).
    pub fn start(dirty_callback: impl Fn() + Send + 'static) -> Self {
        let cache = Arc::new(ArcSwap::from_pointee(DeviceCache::default()));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (dirty_tx, mut dirty_rx) = mpsc::unbounded_channel::<CacheDirty>();
        let shutdown = CancellationToken::new();

        let cache_clone = Arc::clone(&cache);
        let shutdown_clone = shutdown.clone();

        // Spawn a dedicated OS thread that owns its own single-threaded tokio runtime.
        // GTK4 never interacts with this thread except through the types above.
        thread::Builder::new()
            .name("bluetooth-worker".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build bluetooth tokio runtime");

                rt.block_on(async move {
                    // Forward cache-dirty signals to the GTK thread via a callback.
                    // We do this inside the runtime so the dirty_rx poll is on the
                    // worker thread, not the UI thread.
                    let notify = tokio::spawn(async move {
                        while dirty_rx.recv().await.is_some() {
                            dirty_callback();
                        }
                    });

                    worker::run(cache_clone, dirty_tx, cmd_rx, shutdown_clone).await;
                    notify.abort();
                });
            })
            .expect("Failed to spawn bluetooth-worker thread");

        Self {
            cache,
            cmd_tx,
            shutdown,
        }
    }

    /// Read-only snapshot of the current device cache.
    /// The returned `Arc` is always consistent — no partial reads.
    pub fn cache(&self) -> Arc<DeviceCache> {
        self.cache.load_full()
    }

    /// Send a command to the worker thread (fire-and-forget).
    /// Results will arrive via the next cache-dirty notification.
    pub fn send_command(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }
}

impl Drop for BluetoothManager {
    fn drop(&mut self) {
        // Signal the worker thread to exit cleanly.
        self.shutdown.cancel();
    }
}
