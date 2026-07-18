//! Portable plugin-host contract, ported from lensr's `pkg/aihost` core.
//!
//! Defines the host-agnostic building blocks noteit uses to package and install
//! itself as a plugin across CLI hosts (Claude, Codex, Gemini): [`Asset`]s and
//! their registry, the [`Host`] trait plus optional install/status/doctor
//! capabilities, a host factory registry, an atomic tree writer, and
//! synthesized library skills.

pub mod asset;
pub mod host;
pub mod libraries;
pub mod registry;
pub mod write_tree;

pub use asset::{
    Asset, AssetRegistry, DEFAULT_CREATED, Kind, RenderError, TEMPLATE_DELIMS_END,
    TEMPLATE_DELIMS_START, TemplateData,
};
pub use host::{Doctor, DoctorCheck, DoctorReport, Host, Installer, Status};
pub use libraries::{
    AGENT_LIBRARY_SKILL_PATH, COMMAND_LIBRARY_SKILL_PATH, portable_library_skills,
};
pub use registry::{Factory, HostRegistry};
pub use write_tree::{TreeWriter, write_tree_atomic};
