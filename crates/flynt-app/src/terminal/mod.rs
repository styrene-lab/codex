//! Reusable terminal substrate.
//!
//! Keep this module app-agnostic enough to extract for Auspex or a shared
//! Styrene terminal crate. Flynt-specific policy, HostAction review cards, and
//! placement decisions should live outside this module.

pub mod manager;
pub mod types;
pub mod view;

pub use manager::{TerminalManager, TerminalSessionInfo};

pub use types::{
    TERMINAL_CREATE_V1, TerminalCreateParams, TerminalCreateResult, TerminalPlacement,
    TerminalStatus,
};
pub use view::{AlacrittyTerminal, AlacrittyTerminalProps, AlacrittyTerminalSession};
