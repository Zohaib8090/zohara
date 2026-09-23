// bluetooth/mod.rs — Module root. Re-exports the public surface only.

mod connection;
mod manager;
mod types;
mod worker;

pub use manager::BluetoothManager;
pub use types::{BluetoothDevice, Command, DeviceCache};
