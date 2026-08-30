// zohara-connectd: OS-side companion daemon for Zohara OS.
//
// Lifecycle (set by clap in main.rs):
//   1. Read state.json; if `enabled == false`, exit 0 cleanly.
//   2. Generate / load mTLS cert from ~/.config/zohara-connect/.
//   3. Start mDNS advertiser on _zohara-connect._tcp.local @ port 4545.
//   4. Start TCP listener on 0.0.0.0:4545 (and [::]:4545).
//   5. Start Unix-socket IPC on /run/user/$UID/zohara-connectd.sock.
//   6. Subscribe to D-Bus session bus for shutdown signals.
//   7. Block on tokio::join!(...) until any task returns.
//
// Modules will be filled in by subsequent tasks:
//   - protocol.rs   (Task 1.1)
//   - store.rs      (Task 1.2)
//   - tls.rs        (Task 1.3)
//   - pairing.rs    (Task 1.4)
//   - notify.rs     (Task 1.5)
//   - mdns.rs       (Task 1.6)
//   - tcp.rs        (Task 1.7)
//   - ipc.rs        (Task 1.8)
//   - startup.rs    (Task 1.9)

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "zohara-connectd",
    version,
    about = "Zohara OS companion daemon",
    long_about = "Pairs with the Zohara Companion Android app over mTLS, \
                  advertises via mDNS, and bridges phone<->desktop for \
                  clipboard sync, file push, notification mirror, and telemetry."
)]
struct Args {
    /// Run in the foreground (do not daemonize). Useful for debugging.
    #[arg(long)]
    foreground: bool,

    /// Print daemon status (paired devices, recent activity) and exit.
    #[arg(long)]
    status: bool,

    /// Revoke a paired device by its fingerprint and exit.
    #[arg(long, value_name = "FINGERPRINT")]
    revoke: Option<String>,

    /// Enable autostart on next login (default is on; this re-enables it after disable).
    #[arg(long)]
    enable: bool,

    /// Disable autostart. The daemon exits cleanly on next launch and the
    /// systemd user unit is also stopped so it does not restart on login.
    #[arg(long)]
    disable: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging. RUST_LOG defaults to "info"; users can override.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    // CLI subcommands that just talk to a running daemon and exit.
    if args.status {
        // TODO(Task 1.10): implement via IPC `GET_STATUS` command.
        anyhow::bail!("--status not yet implemented (will use IPC GET_STATUS)");
    }
    if let Some(fp) = &args.revoke {
        anyhow::bail!("--revoke {fp} not yet implemented (will use IPC REVOKE)");
    }
    if args.enable {
        anyhow::bail!("--enable not yet implemented (will use IPC ENABLE)");
    }
    if args.disable {
        anyhow::bail!("--disable not yet implemented (will use IPC DISABLE)");
    }

    // Default: run the daemon.
    tracing::info!("zohara-connectd starting up");
    tracing::info!("foreground mode: {}", args.foreground);

    // TODO(Task 1.10): wire up the full pipeline.
    // For now, just print a ready message and block on a future ctrl-c handler.
    tokio::signal::ctrl_c().await?;
    tracing::info!("zohara-connectd shutting down");

    Ok(())
}
