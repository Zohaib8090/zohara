/// NetworkManager backend — single source of truth for all network operations.
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use tokio::time::timeout;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const SHORT_TIMEOUT: Duration = Duration::from_secs(5);

// ── Structured Error Model ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    Timeout,
    NetworkManagerUnavailable,
    AuthFailed(String),
    EnterpriseNotSupported,
    CommandFailed(String),
    ParseError(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::Timeout => write!(f, "Operation timed out"),
            NetworkError::NetworkManagerUnavailable => write!(f, "NetworkManager service is not running"),
            NetworkError::AuthFailed(msg) => write!(f, "Authentication failed: {}", msg),
            NetworkError::EnterpriseNotSupported => write!(f, "Enterprise networks (802.1X) are not supported"),
            NetworkError::CommandFailed(msg) => write!(f, "Network operation failed: {}", msg),
            NetworkError::ParseError(msg) => write!(f, "Failed to parse network data: {}", msg),
        }
    }
}

impl NetworkError {
    pub fn user_message(&self) -> String {
        match self {
            NetworkError::Timeout => "The network operation timed out. Please try again.".to_string(),
            NetworkError::NetworkManagerUnavailable => "Network service is unavailable. Please check system service.".to_string(),
            NetworkError::AuthFailed(msg) if msg.contains("Secrets were required") || msg.contains("bad password") || msg.contains("password") => {
                "Incorrect password or authentication failed. Please try again.".to_string()
            }
            NetworkError::AuthFailed(msg) => format!("Authentication failed: {}", msg),
            NetworkError::EnterpriseNotSupported => "Enterprise networks require 802.1X authentication, which is currently unsupported.".to_string(),
            NetworkError::CommandFailed(msg) => {
                if msg.contains("not found") || msg.contains("No network") {
                    "The specified network could not be found or is out of range.".to_string()
                } else {
                    format!("Network error: {}", msg)
                }
            }
            NetworkError::ParseError(_) => "Received invalid network data from system.".to_string(),
        }
    }
}

// ── Data Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid:     String,
    pub bssid:    String,
    pub security: String,  // e.g. "WPA2", "WPA3", or "" for Open
    pub signal:   u32,     // 0..100
    pub active:   bool,
}

fn security_rank(s: &str) -> u8 {
    let sec = s.to_uppercase();
    if sec.contains("WPA3") {
        3
    } else if sec.contains("WPA2") {
        2
    } else if sec.contains("WPA") {
        1
    } else {
        0
    }
}

impl WifiNetwork {
    pub fn security_label(&self) -> String {
        if self.security.is_empty() || self.security == "--" {
            "Open".to_string()
        } else {
            self.security.clone()
        }
    }

    pub fn is_enterprise(&self) -> bool {
        let sec = self.security.to_uppercase();
        sec.contains("802.1X") || sec.contains("EAP")
    }
}

#[derive(Debug, Clone)]
pub struct EthernetStatus {
    pub device:     String,
    pub state:      String,
    pub connection: String,
}

// ── Escaping Parser for nmcli -t Output ──────────────────────────────────────

/// Properly parses nmcli tabular output (`-t -f ...`).
/// Handles backslash escaping (`\:`, `\\`, `\n`, etc.).
fn parse_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                chars.next();
                current.push(next_ch);
            } else {
                current.push('\\');
            }
        } else if ch == ':' {
            fields.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

// ── Network Operations ───────────────────────────────────────────────────────

/// Trigger a NetworkManager Wi-Fi rescan and await completion with timeout.
pub async fn wifi_rescan() -> Result<(), NetworkError> {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["dev", "wifi", "rescan"]);

    let res = timeout(DEFAULT_TIMEOUT, cmd.status()).await;

    match res {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(NetworkError::CommandFailed(format!("Rescan exited with status {}", status))),
        Ok(Err(e)) => Err(NetworkError::CommandFailed(e.to_string())),
        Err(_) => Err(NetworkError::Timeout),
    }
}

/// Return a deduplicated list of visible Wi-Fi networks (retaining strongest AP per SSID with security preference).
pub async fn wifi_list() -> Result<Vec<WifiNetwork>, NetworkError> {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["-t", "-f", "SSID,BSSID,SECURITY,SIGNAL,ACTIVE", "dev", "wifi", "list"]);

    let res = timeout(DEFAULT_TIMEOUT, cmd.output()).await;

    let out = match res {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(NetworkError::CommandFailed(e.to_string())),
        Err(_) => return Err(NetworkError::Timeout),
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if stderr.contains("NetworkManager is not running") {
            return Err(NetworkError::NetworkManagerUnavailable);
        }
        return Err(NetworkError::CommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut best_by_ssid: HashMap<String, WifiNetwork> = HashMap::new();

    for (line_num, raw_line) in stdout.lines().enumerate() {
        if raw_line.trim().is_empty() {
            continue;
        }

        let parts = parse_terse_line(raw_line);
        if parts.len() < 5 {
            eprintln!("[NetworkBackend] Warning: line {} malformed (expected 5 fields, got {}): {:?}", line_num, parts.len(), raw_line);
            continue;
        }

        let ssid     = parts[0].trim().to_string();
        let bssid    = parts[1].trim().to_string();
        let security = parts[2].trim().to_string();

        let signal: u32 = match parts[3].trim().parse() {
            Ok(s) if s <= 100 => s,
            Ok(s) => {
                eprintln!("[NetworkBackend] Warning: signal out of range ({} > 100) at line {}", s, line_num);
                100
            }
            Err(_) => {
                eprintln!("[NetworkBackend] Warning: invalid signal value '{:?}' at line {}", parts[3], line_num);
                continue;
            }
        };

        let active = parts[4].trim().eq_ignore_ascii_case("yes");

        if ssid.is_empty() {
            continue;
        }

        let candidate = WifiNetwork { ssid: ssid.clone(), bssid, security, signal, active };

        // Deduplication logic: prefer active network, or candidate with stronger signal / higher security rank
        best_by_ssid
            .entry(ssid)
            .and_modify(|existing| {
                if candidate.active
                    || (!existing.active
                        && (candidate.signal > existing.signal + 10
                            || (candidate.signal > existing.signal
                                && security_rank(&candidate.security) >= security_rank(&existing.security))))
                {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }

    let mut networks: Vec<WifiNetwork> = best_by_ssid.into_values().collect();

    networks.sort_by(|a, b| {
        b.active.cmp(&a.active).then(b.signal.cmp(&a.signal))
    });

    Ok(networks)
}

/// Connect to a Wi-Fi network using BSSID or SSID.
pub async fn wifi_connect(ssid: &str, bssid: Option<&str>, password: Option<&str>) -> Result<(), NetworkError> {
    // If BSSID is specified, first attempt connecting with both SSID and BSSID
    if let Some(b) = bssid.filter(|b| !b.is_empty()) {
        let mut cmd = tokio::process::Command::new("nmcli");
        cmd.args(["dev", "wifi", "connect", ssid, "bssid", b]);
        if let Some(pw) = password {
            cmd.args(["password", pw]);
        }

        let res = timeout(DEFAULT_TIMEOUT, cmd.output()).await;
        if let Ok(Ok(out)) = res {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    // Fallback or standard connection by SSID
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["dev", "wifi", "connect", ssid]);
    if let Some(pw) = password {
        cmd.args(["password", pw]);
    }

    let res = timeout(DEFAULT_TIMEOUT, cmd.output()).await;

    let out = match res {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(NetworkError::CommandFailed(e.to_string())),
        Err(_) => return Err(NetworkError::Timeout),
    };

    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let err_msg = if !stderr.is_empty() { stderr } else { stdout };

        if err_msg.contains("Secrets were required")
            || err_msg.contains("password")
            || err_msg.contains("authentication")
            || err_msg.contains("denied")
        {
            Err(NetworkError::AuthFailed(err_msg))
        } else {
            Err(NetworkError::CommandFailed(err_msg))
        }
    }
}

/// Return state of Ethernet interfaces.
pub async fn ethernet_status() -> Result<Vec<EthernetStatus>, NetworkError> {
    let mut cmd = tokio::process::Command::new("nmcli");
    cmd.args(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"]);

    let res = timeout(SHORT_TIMEOUT, cmd.output()).await;

    let out = match res {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(NetworkError::CommandFailed(e.to_string())),
        Err(_) => return Err(NetworkError::Timeout),
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(NetworkError::CommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut results = Vec::new();

    for raw in stdout.lines() {
        let parts = parse_terse_line(raw);
        if parts.len() < 4 { continue; }
        if parts[1].trim() != "ethernet" { continue; }

        results.push(EthernetStatus {
            device:     parts[0].trim().to_string(),
            state:      parts[2].trim().to_string(),
            connection: parts[3].trim().to_string(),
        });
    }

    Ok(results)
}
