//! Product-agnostic Windows BLE/WinRT helpers for Denzic host applications.
//!
//! Device names, service UUIDs, signatures, persistence, and product recovery
//! policy stay in the calling product adapter; this crate only carries the
//! generic Windows Bluetooth LE building blocks (async waits, GATT I/O,
//! advertisement scanning, PnP enumeration) plus the platform-neutral BLE
//! failure taxonomy. Only the pure-Rust modules ([`address`], [`failure`])
//! are available off Windows; every WinRT helper is Windows-only.

mod generated;

pub mod address;
pub mod failure;

pub use address::*;
pub use generated::*;

#[cfg(target_os = "windows")]
mod device;
#[cfg(target_os = "windows")]
mod gatt;
#[cfg(target_os = "windows")]
mod pnp;
#[cfg(target_os = "windows")]
mod scan;
#[cfg(target_os = "windows")]
mod wait;

#[cfg(target_os = "windows")]
pub use device::*;
#[cfg(target_os = "windows")]
pub use gatt::*;
#[cfg(target_os = "windows")]
pub use pnp::*;
#[cfg(target_os = "windows")]
pub use scan::*;
#[cfg(target_os = "windows")]
pub use wait::*;
