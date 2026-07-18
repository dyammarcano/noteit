//! Portable plugin-host contract, ported from an internal Go plugin-host package.
//!
//! Defines the host-agnostic building blocks noteit uses to package and install
//! itself as a plugin across CLI hosts (Claude, Codex, Gemini): [`Asset`]s and
//! their registry, the [`Host`] trait plus optional install/status/doctor
//! capabilities, a host factory registry, an atomic tree writer, and
//! synthesized library skills.

pub mod asset;
pub mod command;
pub mod host;
pub mod hosts;
pub mod libraries;
pub mod noteit;
pub mod registry;
pub mod write_tree;

pub use asset::{
    Asset, AssetRegistry, DEFAULT_CREATED, Kind, RenderError, TEMPLATE_DELIMS_END,
    TEMPLATE_DELIMS_START, TemplateData,
};
/// Serializes tests that mutate the process-global `NOTEIT_PLUGIN_ROOT` env
/// var, which cargo would otherwise run concurrently (a data race). No
/// non-test code reads that var off the main path, so locking the mutators is
/// sufficient.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub use command::{HostSel, PluginCmd};
pub use host::{Doctor, DoctorCheck, DoctorReport, Host, Installer};
pub use hosts::NoteitHost;
pub use libraries::{
    AGENT_LIBRARY_SKILL_PATH, COMMAND_LIBRARY_SKILL_PATH, portable_library_skills,
};
pub use registry::{Factory, HostRegistry};
pub use write_tree::{TreeWriter, write_tree_atomic};
