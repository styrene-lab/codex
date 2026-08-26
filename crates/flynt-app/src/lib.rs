#![allow(
    clippy::collapsible_if,
    clippy::clone_on_copy,
    clippy::large_enum_variant,
    clippy::let_underscore_future,
    clippy::map_identity,
    clippy::mutex_atomic,
    clippy::ptr_arg,
    clippy::question_mark,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

pub mod acp;
pub mod app;
pub mod apple_notes;
pub mod armory_install;
pub mod armory_resolution;
pub mod bootstrap;
pub mod build_identity;
pub mod components;
pub mod daemon_manager;
pub mod design_board_assets;
pub mod design_board_capture;
pub mod design_focus;
pub mod excalidraw_preview;
pub mod host_actions;
pub mod icons;
pub mod menu;
pub mod native_invocation_execute;
pub mod omegon_activation;
pub mod omegon_cli_contract;
pub mod omegon_cli_probe;
pub mod omegon_deployment_diagnostics;
pub mod omegon_setup;
pub mod project_registry_commands;
pub mod push_pipeline;
pub mod self_update;
pub mod state;
pub mod sync_prereq;
pub mod terminal;
pub mod theme;
pub mod ui_state;
pub mod views;
pub mod visual_artifact_open;
pub mod visual_artifact_surface;
