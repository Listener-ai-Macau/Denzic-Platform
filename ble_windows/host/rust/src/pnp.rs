//! Hidden PowerShell process helpers and PnP device enumeration.
//!
//! Product diagnostics must never spawn a visible console window or the
//! legacy Windows PowerShell 5.1 host: every script runs through `pwsh
//! -NoProfile -Command` with `CREATE_NO_WINDOW`. The enumeration scripts
//! return raw FriendlyName/InstanceId pairs; product adapters apply their own
//! name/signature filters to the entries.

use std::os::windows::process::CommandExt;
use std::process::{Command, Output};

use serde::Deserialize;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CONFIGRET, CR_ACCESS_DENIED, CR_NO_SUCH_DEVINST, CR_NO_SUCH_DEVNODE, CR_QUERY_VETOED,
    CR_REMOVE_VETOED, PNP_VETO_TYPE,
};

use crate::CREATE_NO_WINDOW_FLAG;

/// `CREATE_NO_WINDOW` process creation flag.
pub const WINDOWS_CREATE_NO_WINDOW: u32 = CREATE_NO_WINDOW_FLAG;

/// Get-PnpDevice enumeration of BLE/HID device nodes (JSON on stdout).
pub const PNP_BLE_HID_ENUM_SCRIPT: &str = r#"
$ProgressPreference = 'SilentlyContinue'
Get-PnpDevice -ErrorAction SilentlyContinue |
  Where-Object { $_.InstanceId -match '^(BTHLE|BTHLEDEVICE|HID)\\' } |
  Select-Object FriendlyName,InstanceId |
  ConvertTo-Json -Compress
"#;

/// Win32_PnPEntity enumeration of *present* BLE/HID device nodes.
pub const PNP_PRESENT_BLE_HID_ENUM_SCRIPT: &str = r#"
$ProgressPreference = 'SilentlyContinue'
Get-CimInstance Win32_PnPEntity -Filter "DeviceID LIKE 'BTHLE%' OR DeviceID LIKE 'BTHLEDEVICE%' OR DeviceID LIKE 'HID%'" -ErrorAction SilentlyContinue |
  Where-Object { $_.Present -ne $false } |
  Select-Object @{ Name = 'FriendlyName'; Expression = { $_.Name } }, @{ Name = 'InstanceId'; Expression = { $_.DeviceID } } |
  ConvertTo-Json -Compress
"#;

/// Raw PnP device entry as emitted by the enumeration scripts.
#[derive(Debug, Clone, Deserialize)]
pub struct PnpDeviceEntry {
    #[serde(rename = "FriendlyName")]
    pub friendly_name: Option<String>,
    #[serde(rename = "InstanceId")]
    pub instance_id: Option<String>,
}

pub fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.creation_flags(WINDOWS_CREATE_NO_WINDOW);
    command
}

pub fn hidden_pwsh_command() -> Command {
    hidden_command("pwsh")
}

pub fn run_hidden_pwsh_script(script: &str, label: &str) -> Result<Output, String> {
    let output = hidden_pwsh_command()
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|err| format!("start pwsh for {label} failed: {err}"))?;
    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("pwsh for {label} exited with status {}", output.status)
    } else {
        format!(
            "pwsh for {label} exited with status {}: {stderr}",
            output.status
        )
    })
}

/// Run a PnP enumeration script and decode its JSON output into raw entries.
/// An empty stdout means "no devices", and a single JSON object is treated
/// as a one-element list (ConvertTo-Json collapses single results).
pub fn query_pnp_device_entries(script: &str, label: &str) -> Result<Vec<PnpDeviceEntry>, String> {
    let output = run_hidden_pwsh_script(script, label)?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|err| format!("parse {label} JSON failed: {err}; output={stdout}"))?;
    let raw_entries = match value {
        serde_json::Value::Array(values) => values,
        serde_json::Value::Null => Vec::new(),
        other => vec![other],
    };

    let mut entries = Vec::new();
    for value in raw_entries {
        let entry: PnpDeviceEntry = serde_json::from_value(value)
            .map_err(|err| format!("decode {label} entry failed: {err}"))?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Enumerate all BLE/HID PnP device nodes (present or not).
pub fn enumerate_ble_hid_pnp_entries() -> Result<Vec<PnpDeviceEntry>, String> {
    query_pnp_device_entries(PNP_BLE_HID_ENUM_SCRIPT, "Get-PnpDevice BLE/HID enumeration")
}

/// Enumerate only the currently present BLE/HID PnP device nodes.
pub fn enumerate_present_ble_hid_pnp_entries() -> Result<Vec<PnpDeviceEntry>, String> {
    query_pnp_device_entries(
        PNP_PRESENT_BLE_HID_ENUM_SCRIPT,
        "Get-CimInstance present BLE/HID enumeration",
    )
}

/// Human-readable detail for a CfgMgr32 status (+ optional remove veto).
pub fn configret_detail(status: CONFIGRET, veto: Option<(&PNP_VETO_TYPE, &[u16])>) -> String {
    let label = if status == CR_ACCESS_DENIED {
        "access denied"
    } else if status == CR_REMOVE_VETOED {
        "remove vetoed"
    } else if status == CR_QUERY_VETOED {
        "query vetoed"
    } else if status == CR_NO_SUCH_DEVINST || status == CR_NO_SUCH_DEVNODE {
        "device node not found"
    } else {
        "configuration manager error"
    };
    let mut detail = format!("{label} ({status:?})");
    if let Some((veto_type, veto_name)) = veto {
        let end = veto_name
            .iter()
            .position(|ch| *ch == 0)
            .unwrap_or(veto_name.len());
        let veto_name = String::from_utf16_lossy(&veto_name[..end]);
        if !veto_name.trim().is_empty() || veto_type.0 != 0 {
            detail.push_str(&format!(
                ", veto_type={veto_type:?}, veto_name={}",
                veto_name.trim()
            ));
        }
    }
    detail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_pwsh_command_targets_pwsh_not_windows_powershell() {
        let command = hidden_pwsh_command();
        assert_eq!(command.get_program().to_string_lossy(), "pwsh");
        let command = hidden_command("cmd");
        assert_eq!(command.get_program().to_string_lossy(), "cmd");
    }

    #[test]
    fn configret_detail_labels_known_statuses() {
        assert!(configret_detail(CR_ACCESS_DENIED, None).contains("access denied"));
        assert!(configret_detail(CR_REMOVE_VETOED, None).contains("remove vetoed"));
        assert!(configret_detail(CR_NO_SUCH_DEVNODE, None).contains("device node not found"));
        let veto_type = PNP_VETO_TYPE(2);
        let veto_name: Vec<u16> = "Veto".encode_utf16().chain(std::iter::once(0)).collect();
        let detail = configret_detail(CR_QUERY_VETOED, Some((&veto_type, &veto_name)));
        assert!(detail.contains("query vetoed"));
        assert!(detail.contains("veto_name=Veto"));
    }
}
