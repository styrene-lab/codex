use crate::omegon_deployment_diagnostics::{
    DeploymentDiagnostic, DeploymentManifestSource, LoadedDeploymentManifest,
    classify_loaded_deployment,
};
use crate::self_update::UpdateChannel;
use crate::{
    acp::AcpSession,
    bootstrap::{AppContext, OmegonRuntimeContext, PendingProjectSetup},
    components::daemon_settings::DaemonSettingsSection,
    components::identity_settings::IdentitySettingsSection,
    components::provider_settings::ProviderSettingsSection,
    state::{SettingsCategory, SettingsPage, ThemeName},
    theme::{ThemeLibrary, UiTheme, import_tweakcn_theme, import_tweakcn_theme_from_locator},
    views::{IndexingScopesEditor, PublicationRulesEditor},
};
use dioxus::prelude::*;
use std::rc::Rc;
use flynt_core::models::{
    AppearanceConfig, FlyntOperatorSettings, FontSizePreset, IndexingConfig, LocalRuntimeConfig,
    OmegonProfile, ProjectConfig, SyncConfig, VisualizationConfig,
};

// ── Settings view ─────────────────────────────────────────────────────────────

#[component]
pub fn SettingsView() -> Element {
    let ctx = use_context::<AppContext>();
    let loaded_deployment = load_deployment_for_settings(&ctx.omegon());
    let deployment_diagnostic = classify_loaded_deployment(
        &loaded_deployment,
        ctx.deployment_metadata().as_ref(),
        &ctx.project_root(),
    );
    let armory_report = crate::armory_resolution::resolve_deployment_skills(
        &loaded_deployment.manifest,
        &ctx.project_root(),
        &ctx.omegon().home_dir,
        None,
    );
    let custom_skill_id = use_signal(String::new);
    let mut armory_install_msg = use_signal(|| Option::<String>::None);
    let mut armory_install_refresh = use_signal(|| 0u64);
    let _ = armory_install_refresh.read();
    let cli_probe = ctx.omegon_cli_probe();
    let probe_ctx = ctx.clone();
    use_effect(move || {
        if probe_ctx.omegon_cli_probe().is_none() {
            let probe_ctx = probe_ctx.clone();
            spawn(async move {
                let binary = probe_ctx.omegon().resolve_binary();
                let result = crate::omegon_cli_probe::probe_omegon_cli(binary).await;
                probe_ctx.set_omegon_cli_probe(result);
            });
        }
    });

    // Appearance — reactive, applied immediately via context signals.
    let mut theme = use_context::<Signal<ThemeName>>();
    let mut font_sz = use_context::<Signal<FontSizePreset>>();
    let mut theme_library = use_context::<Signal<ThemeLibrary>>();
    let mut operator_settings_state = use_context::<Signal<FlyntOperatorSettings>>();

    // Project + sync — local form state; persisted on explicit Save.
    let mut project_name = use_signal(|| ctx.project().config.project_name.clone());
    let mut sync_config = use_signal(|| ctx.project().config.sync.clone());
    let mut local_state_root = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .local_state_root
            .as_ref()
            .map(|path: &std::path::PathBuf| path.display().to_string())
            .unwrap_or_default()
    });
    let mut flynt_index_db_path = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .flynt_index_db_path
            .as_ref()
            .map(|path: &std::path::PathBuf| path.display().to_string())
            .unwrap_or_default()
    });
    let mut omegon_runtime_root = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .omegon_runtime_root
            .as_ref()
            .map(|path: &std::path::PathBuf| path.display().to_string())
            .unwrap_or_default()
    });
    let mut omegon_mind_db_path = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .omegon_mind_db_path
            .as_ref()
            .map(|path: &std::path::PathBuf| path.display().to_string())
            .unwrap_or_default()
    });
    let mut omegon_channel =
        use_signal(|| ctx.project().config.local_runtime.omegon_channel.clone());
    let mut omegon_bin_override = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .omegon_bin_override
            .clone()
            .unwrap_or_default()
    });
    let mut styrene_identity_profile = use_signal(|| {
        ctx.project()
            .config
            .local_runtime
            .styrene_identity_profile
            .clone()
            .unwrap_or_default()
    });
    let mut flynt_update_channel =
        use_signal(|| OmegonRuntimeContext::load_launcher_profile().flynt_update_channel);

    let publication_default_visibility =
        use_signal(|| ctx.project().config.publication.default_visibility);
    let publication_rules = use_signal(|| ctx.project().config.publication.rules.clone());

    let _project_profile_state = use_context::<Signal<OmegonProfile>>();
    // Indexing
    let mut write_frontmatter = use_signal(|| ctx.project().config.indexing.write_frontmatter);
    let mut track_index_snapshot =
        use_signal(|| ctx.project().config.indexing.track_index_snapshot);
    let indexing_scopes = use_signal(|| ctx.project().config.indexing.scopes.clone());

    // Raw config editor
    let mut show_raw_config = use_signal(|| false);
    let config_path = ctx.project_root().join(".flynt/config.toml");
    let mut raw_config_text = use_signal(|| {
        std::fs::read_to_string(ctx.project_root().join(".flynt/config.toml")).unwrap_or_default()
    });
    let mut raw_config_msg = use_signal(|| Option::<(&'static str, &'static str)>::None);

    // Visualization
    let mut excalidraw_auto_export =
        use_signal(|| ctx.project().config.visualization.excalidraw_auto_export);
    let mut d2_auto_render = use_signal(|| ctx.project().config.visualization.d2_auto_render);
    let mut d2_theme = use_signal(|| ctx.project().config.visualization.d2_theme.to_string());
    let mut d2_layout = use_signal(|| ctx.project().config.visualization.d2_layout.clone());
    let mut d2_bin = use_signal(|| {
        ctx.project()
            .config
            .visualization
            .d2_bin
            .clone()
            .unwrap_or_default()
    });

    // Daemon config — managed by DaemonSettingsSection
    let daemon_config = use_signal(|| ctx.omegon().load_operator_settings().agent_daemon.clone());

    let mut save_msg = use_signal(|| Option::<(&'static str, &'static str)>::None);
    let registry_msg = use_signal(|| Option::<(&'static str, String)>::None);
    let mut import_theme_msg = use_signal(|| Option::<(&'static str, String)>::None);
    let mut theme_url = use_signal(String::new);
    let publish_msg = use_signal(|| Option::<(&'static str, String)>::None);

    let mut active_page = use_context::<Signal<SettingsPage>>();
    let shared_session = use_context::<Signal<Option<Rc<AcpSession>>>>();

    let project = ctx.project();
    let omegon = ctx.omegon();
    let omegon_for_save = omegon.clone();
    let omegon_for_file_theme_import = omegon.clone();
    let omegon_for_remote_theme_import = omegon.clone();
    let publish_project = ctx.project();
    let mut publish_msg_signal = publish_msg;
    let publish_preview =
        move |_| match OmegonRuntimeContext::export_publication_preview(&publish_project) {
            Ok(output_path) => {
                let mut profile = OmegonRuntimeContext::load_launcher_profile();
                let target = OmegonRuntimeContext::publication_target(&publish_project);
                profile.pending_setup = Some(PendingProjectSetup::PublishPreview {
                    output_path: output_path.clone(),
                    repo: target
                        .as_ref()
                        .map(|target| target.repo.clone())
                        .unwrap_or_default(),
                    branch: target
                        .as_ref()
                        .map(|target| target.branch.clone())
                        .unwrap_or_default(),
                });
                let _ = OmegonRuntimeContext::save_launcher_profile(&profile);
                *publish_msg_signal.write() = Some((
                    "ok",
                    format!("Local preview exported to {}", output_path.display()),
                ));
            }
            Err(err) => {
                *publish_msg_signal.write() =
                    Some(("err", format!("Publish preview failed: {err}")));
            }
        };
    let save = move |_| {
        // Validate git sync config
        if let flynt_core::models::SyncConfig::Git {
            ref remote,
            ref branch,
            ..
        } = *sync_config.read()
        {
            if remote.trim().is_empty() {
                *save_msg.write() = Some(("err", "Git remote name cannot be empty."));
                return;
            }
            if branch.trim().is_empty() {
                *save_msg.write() = Some(("err", "Git branch name cannot be empty."));
                return;
            }
        }

        // Validate paths before saving
        for (_label, val) in [
            ("Local state root", local_state_root.read().clone()),
            ("Index DB path", flynt_index_db_path.read().clone()),
            ("Omegon runtime root", omegon_runtime_root.read().clone()),
            ("Omegon mind DB path", omegon_mind_db_path.read().clone()),
        ] {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                let p = std::path::Path::new(&trimmed);
                if !p.is_absolute() {
                    *save_msg.write() = Some(("err", "Paths must be absolute (start with /)."));
                    return;
                }
            }
        }

        let local_runtime = LocalRuntimeConfig {
            local_state_root: path_from_input(local_state_root.read().as_str()),
            flynt_index_db_path: path_from_input(flynt_index_db_path.read().as_str()),
            omegon_runtime_root: path_from_input(omegon_runtime_root.read().as_str()),
            omegon_mind_db_path: path_from_input(omegon_mind_db_path.read().as_str()),
            styrene_identity_profile: string_from_input(styrene_identity_profile.read().as_str()),
            omegon_serve_host: None,
            omegon_channel: omegon_channel.read().clone(),
            omegon_bin_override: string_from_input(omegon_bin_override.read().as_str()),
        };
        let config = ProjectConfig {
            project_name: project_name.read().clone(),
            sync: sync_config.read().clone(),
            appearance: AppearanceConfig {
                theme: theme.read().0.clone(),
                font_size: *font_sz.read(),
            },
            local_runtime,
            publication: flynt_core::models::PublicationPolicy {
                default_visibility: *publication_default_visibility.read(),
                rules: publication_rules.read().clone(),
            },
            security: ctx.project().config.security.clone(),
            indexing: IndexingConfig {
                write_frontmatter: *write_frontmatter.read(),
                scopes: indexing_scopes.read().clone(),
                track_index_snapshot: *track_index_snapshot.read(),
            },
            visualization: VisualizationConfig {
                excalidraw_auto_export: *excalidraw_auto_export.read(),
                d2_auto_render: *d2_auto_render.read(),
                d2_theme: d2_theme.read().parse::<u32>().unwrap_or(200),
                d2_layout: d2_layout.read().clone(),
                d2_bin: {
                    let bin = d2_bin.read().trim().to_string();
                    if bin.is_empty() { None } else { Some(bin) }
                },
            },
        };

        // Check if sync backend changed — trigger project migration
        let old_sync = &project.config.sync;
        let new_sync = &config.sync;
        if old_sync != new_sync {
            let project_name = config.project_name.clone();
            let current_root = project.root.clone();
            let sync_for_migrate = new_sync.clone();
            match flynt_store::migrate::migrate_project(
                &current_root,
                &project_name,
                &sync_for_migrate,
                false,
            ) {
                Ok(result) => {
                    if result.new_root != current_root {
                        // Project moved — update launcher profile and switch runtime
                        let mut profile =
                            crate::bootstrap::OmegonRuntimeContext::load_launcher_profile();
                        crate::bootstrap::OmegonRuntimeContext::register_known_project(
                            &mut profile,
                            &result.new_root,
                            &project_name,
                        );
                        let _ =
                            crate::bootstrap::OmegonRuntimeContext::save_launcher_profile(&profile);
                        let mut migrate_ctx = ctx;
                        migrate_ctx.set_runtime(crate::bootstrap::runtime_state_for_project_root(
                            result.new_root,
                        ));
                        *save_msg.write() = Some(("ok", "Project migrated and sync updated."));
                        return; // config already written by migrate
                    }
                    // Same location — migration updated config in place, continue to save other settings
                }
                Err(e) => {
                    tracing::error!("Migration failed: {e}");
                    *save_msg.write() = Some(("err", "Migration failed — check logs."));
                    return;
                }
            }
        }

        match project.save_config(&config) {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("save_config: {e}");
                *save_msg.write() = Some(("err", "Save failed — check logs."));
                return;
            }
        }

        // Persist daemon config alongside project config
        let mut operator = omegon_for_save.load_operator_settings();
        operator.agent_daemon = daemon_config.read().clone();
        operator.ui_theme.active_theme = theme.read().0.clone();
        operator.ui_theme.imported_themes = theme_library.read().imported_for_settings();
        if let Err(e) = omegon_for_save.save_operator_settings(&operator) {
            tracing::error!("save_operator_settings: {e}");
            *save_msg.write() = Some(("err", "Operator settings save failed — check logs."));
            return;
        }
        *operator_settings_state.write() = operator;

        let mut profile = OmegonRuntimeContext::load_launcher_profile();
        profile.flynt_update_channel = *flynt_update_channel.read();
        if let Err(e) = OmegonRuntimeContext::save_launcher_profile(&profile) {
            tracing::error!("save_launcher_profile: {e}");
            *save_msg.write() = Some(("err", "Launcher settings save failed — check logs."));
            return;
        }

        *save_msg.write() = Some(("ok", "Settings saved."));
    };

    rsx! {
        div { class: "settings-root settings-root-split",
            // ── Sidebar ──────────────────────────────────────────────────
            // Hierarchical category → page navigation. Categories with
            // a single page (General, Project, Advanced) render as a
            // direct link. Categories with multiple pages (Omegon)
            // render as a header with nested children.
            nav { class: "settings-sidebar",
                for cat in SettingsCategory::all() {
                    {
                        let pages = SettingsPage::in_category(*cat);
                        let single = pages.len() == 1;
                        if single {
                            let page = pages[0];
                            let is_active = *active_page.read() == page;
                            rsx! {
                                button {
                                    class: if is_active { "settings-nav-item active" } else { "settings-nav-item" },
                                    onclick: move |_| *active_page.write() = page,
                                    "{cat.label()}"
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "settings-nav-group",
                                    div { class: "settings-nav-group-header", "{cat.label()}" }
                                    for page in pages {
                                        {
                                            let is_active = *active_page.read() == page;
                                            rsx! {
                                                button {
                                                    class: if is_active { "settings-nav-item nested active" } else { "settings-nav-item nested" },
                                                    onclick: move |_| *active_page.write() = page,
                                                    "{page.label()}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-scroll",
                if active_page.read().requires_live_omegon_session() && shared_session.read().is_none() {
                    LiveOmegonSessionRequired { page: *active_page.read() }
                } else {

                // ════════════════════════════════════════════════════════════
                // General → Appearance: theme + font size
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::GeneralAppearance {

                SettingsSection { heading: "Appearance",
                    SettingsRow {
                        label: "Theme",
                        hint: "Visual theme applied across the sidebar, editor, Design Board, and rendered preview.",
                        div { class: "theme-stack",
                            div { class: "theme-actions",
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| {
                                        let Some(path) = rfd::FileDialog::new()
                                            .add_filter("tweak.cn theme", &["json"])
                                            .pick_file()
                                        else {
                                            return;
                                        };

                                        match std::fs::read_to_string(&path)
                                            .map_err(anyhow::Error::from)
                                            .and_then(|content| import_tweakcn_theme(&content))
                                        {
                                            Ok(imported) => {
                                                let imported_id = theme_library.write().upsert_imported(imported);
                                                *theme.write() = ThemeName(imported_id.clone());

                                                let imported_themes = theme_library.read().imported_for_settings();
                                                let operator_to_save = {
                                                    let mut operator = operator_settings_state.write();
                                                    operator.ui_theme.active_theme = imported_id;
                                                    operator.ui_theme.imported_themes = imported_themes;
                                                    operator.clone()
                                                };
                                                match omegon_for_file_theme_import.save_operator_settings(&operator_to_save) {
                                                    Ok(()) => {
                                                        *import_theme_msg.write() = Some(("ok", format!("Imported {}", path.display())));
                                                    }
                                                    Err(err) => {
                                                        *import_theme_msg.write() = Some(("err", format!("Theme imported but save failed: {err}")));
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                *import_theme_msg.write() = Some(("err", format!("Theme import failed: {err}")));
                                            }
                                        }
                                    },
                                    "Import tweak.cn JSON"
                                }
                                input {
                                    class: "input settings-input theme-import-input",
                                    placeholder: "theme URL, registry slug, or theme ID",
                                    value: "{theme_url}",
                                    oninput: move |e| *theme_url.write() = e.value(),
                                }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| {
                                        let locator = theme_url.read().trim().to_string();
                                        if locator.is_empty() {
                                            *import_theme_msg.write() = Some(("err", "Enter a theme URL, registry slug, or theme ID.".into()));
                                            return;
                                        }

                                        let mut theme_library = theme_library;
                                        let mut theme = theme;
                                        let mut operator_settings_state = operator_settings_state;
                                        let omegon = omegon_for_remote_theme_import.clone();
                                        let mut import_theme_msg = import_theme_msg;
                                        spawn(async move {
                                            *import_theme_msg.write() = Some(("ok", "Fetching theme…".into()));
                                            match import_tweakcn_theme_from_locator(&locator).await {
                                                Ok(imported) => {
                                                    let imported_id = theme_library.write().upsert_imported(imported);
                                                    *theme.write() = ThemeName(imported_id.clone());

                                                    let imported_themes = theme_library.read().imported_for_settings();
                                                    let operator_to_save = {
                                                        let mut operator = operator_settings_state.write();
                                                        operator.ui_theme.active_theme = imported_id;
                                                        operator.ui_theme.imported_themes = imported_themes;
                                                        operator.clone()
                                                    };
                                                    match omegon.save_operator_settings(&operator_to_save) {
                                                        Ok(()) => {
                                                            *import_theme_msg.write() = Some(("ok", "Imported remote theme.".into()));
                                                        }
                                                        Err(err) => {
                                                            *import_theme_msg.write() = Some(("err", format!("Theme imported but save failed: {err}")));
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    *import_theme_msg.write() = Some(("err", format!("Remote theme import failed: {err}")));
                                                }
                                            }
                                        });
                                    },
                                    "Add theme"
                                }
                                if let Some((kind, msg)) = import_theme_msg.read().as_ref() {
                                    span { class: "settings-inline-msg {kind}", "{msg}" }
                                }
                            }
                            div { class: "theme-grid",
                            for entry in theme_library.read().themes.clone() {
                                {
                                    let active = theme.read().0 == entry.id;
                                    let omegon_for_theme_select = omegon.clone();
                                    rsx! {
                                ThemeCard {
                                    entry,
                                    active,
                                    on_select: move |id: String| {
                                        *theme.write() = ThemeName(id.clone());
                                        let operator_to_save = {
                                            let mut operator = operator_settings_state.write();
                                            operator.ui_theme.active_theme = id;
                                            operator.ui_theme.imported_themes = theme_library.read().imported_for_settings();
                                            operator.clone()
                                        };
                                        if let Err(err) = omegon_for_theme_select.save_operator_settings(&operator_to_save) {
                                            *import_theme_msg.write() = Some(("err", format!("Theme save failed: {err}")));
                                        }
                                    },
                                }
                                    }
                                }
                            }
                            }
                        }
                    }
                    SettingsRow {
                        label: "Font size",
                        hint: "Base font scale for the editor and rendered preview. Sidebar and chrome text stay fixed.",
                        div { class: "font-size-row",
                            for preset in [FontSizePreset::Small, FontSizePreset::Medium,
                                           FontSizePreset::Large, FontSizePreset::XLarge] {
                                button {
                                    class: if *font_sz.read() == preset { "font-size-btn active" } else { "font-size-btn" },
                                    onclick: move |_| *font_sz.write() = preset,
                                    "{preset.label()}"
                                }
                            }
                        }
                    }
                }

                } // end Appearance

                // ════════════════════════════════════════════════════════════
                // General → Sync: backend picker + git config (if Git is active)
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::GeneralSync {

                SettingsSection { heading: "Sync",
                    SettingsRow {
                        label: "Backend",
                        hint: "How flynt mirrors this project off-device. Hover an option to see what it does and any prerequisites you need to satisfy first.",
                        {
                            // Evaluate prerequisites once per render so each
                            // radio knows whether it should be selectable,
                            // disabled, or selectable-with-warning. Reading
                            // the project root reactively isn't required —
                            // it doesn't change without a project switch
                            // (which would re-mount the settings panel).
                            let project_root = ctx.project_root();
                            let icloud_status = crate::sync_prereq::evaluate_icloud(&project_root);
                            let git_status = crate::sync_prereq::evaluate_git();
                            let blocking_msg = if matches!(*sync_config.read(), SyncConfig::ICloud)
                                && icloud_status.is_blocked() {
                                icloud_status.explanation().map(String::from)
                            } else if matches!(*sync_config.read(), SyncConfig::Git { .. })
                                && git_status.is_blocked() {
                                git_status.explanation().map(String::from)
                            } else {
                                None
                            };
                            rsx! {
                                div { class: "radio-group",
                                    SyncRadio {
                                        label: "None",
                                        description: "Flynt won't push anywhere. Notes live only on this machine.",
                                        active: matches!(*sync_config.read(), SyncConfig::None),
                                        status: crate::sync_prereq::evaluate_none(),
                                        on_select: move |_| *sync_config.write() = SyncConfig::None,
                                    }
                                    SyncRadio {
                                        label: "iCloud",
                                        description: "Sync via iCloud Drive. macOS-only; requires the project folder to live inside iCloud Drive.",
                                        active: matches!(*sync_config.read(), SyncConfig::ICloud),
                                        status: icloud_status.clone(),
                                        on_select: move |_| *sync_config.write() = SyncConfig::ICloud,
                                    }
                                    SyncRadio {
                                        label: "Git",
                                        description: "Auto-commit and push to a git remote. Requires git installed and a configured provider token (GitHub, GitLab, etc).",
                                        active: matches!(*sync_config.read(), SyncConfig::Git { .. }),
                                        status: git_status.clone(),
                                        on_select: move |_| *sync_config.write() = SyncConfig::Git {
                                            remote: "origin".into(),
                                            branch: "main".into(),
                                            auto_commit_seconds: 60,
                                        },
                                    }
                                }
                                if let Some(msg) = blocking_msg {
                                    div { class: "settings-prereq-warning",
                                        span { class: "settings-prereq-warning-icon", "\u{26A0}" }
                                        span { "{msg}" }
                                    }
                                }
                            }
                        }
                    }
                    if let SyncConfig::Git { remote, branch, auto_commit_seconds } = sync_config.read().clone() {
                        SettingsRow { label: "Remote URL",
                            input {
                                class: "input settings-input",
                                r#type: "text",
                                value: "{remote}",
                                oninput: move |e| {
                                    if let SyncConfig::Git { ref mut remote, .. } = *sync_config.write() {
                                        *remote = e.value();
                                    }
                                },
                            }
                        }
                        SettingsRow { label: "Branch",
                            input {
                                class: "input settings-input",
                                r#type: "text",
                                value: "{branch}",
                                oninput: move |e| {
                                    if let SyncConfig::Git { ref mut branch, .. } = *sync_config.write() {
                                        *branch = e.value();
                                    }
                                },
                            }
                        }
                        SettingsRow { label: "Auto-commit (sec)",
                            input {
                                class: "input settings-input settings-input-narrow",
                                r#type: "number",
                                min: "0",
                                value: "{auto_commit_seconds}",
                                oninput: move |e| {
                                    let secs: u64 = e.value().parse().unwrap_or(0);
                                    let secs = if secs > 0 && secs < 30 { 30 } else { secs };
                                    if let SyncConfig::Git { ref mut auto_commit_seconds, .. } = *sync_config.write() {
                                        *auto_commit_seconds = secs;
                                    }
                                },
                            }
                            span { class: "settings-hint muted", "(0 = manual only, minimum 30)" }
                        }
                        {
                            let provider_id = flynt_core::providers::provider_for_url(&remote);
                            let cred_status = provider_id.and_then(|pid| {
                                flynt_core::providers::PROVIDERS.iter().find(|p| p.id == pid)
                            }).map(|p| flynt_core::providers::probe_provider(p));
                            // Shared deep-link to the Providers page for the
                            // operator to manage the credential. Same control
                            // for both authenticated + missing cases — the
                            // message changes, the destination doesn't.
                            let mut goto_providers = active_page;
                            let providers_link = rsx! {
                                button {
                                    class: "btn btn-ghost btn-sm settings-providers-link",
                                    onclick: move |_| {
                                        *goto_providers.write() = SettingsPage::OmegonProviders;
                                    },
                                    "Manage tokens \u{2192}"
                                }
                            };
                            match (provider_id, cred_status) {
                                (Some(pid), Some(flynt_core::providers::CredentialStatus::Authenticated { source })) => rsx! {
                                    SettingsRow { label: "Git credentials",
                                        div { class: "settings-row-inline",
                                            span { class: "provider-status authenticated" }
                                            span { class: "provider-status-text", "Authenticated ({source}) — {pid}" }
                                            {providers_link}
                                        }
                                    }
                                },
                                (Some(pid), _) => rsx! {
                                    SettingsRow { label: "Git credentials",
                                        div { class: "settings-row-inline",
                                            span { class: "provider-status missing" }
                                            span { class: "provider-status-text", "No token for {pid} yet — push will fail until you add one" }
                                            {providers_link}
                                        }
                                    }
                                },
                                _ => rsx! {
                                    SettingsRow { label: "Git credentials",
                                        div { class: "settings-row-inline",
                                            span { class: "settings-hint muted", "Unknown host — credentials managed by system git" }
                                            {providers_link}
                                        }
                                    }
                                },
                            }
                        }
                    }
                }

                } // end Sync

                // ════════════════════════════════════════════════════════════
                // General → Identity: passphrase / biometric identity setup
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::GeneralIdentity {
                    IdentitySettingsSection {}
                }

                // ════════════════════════════════════════════════════════════
                // General → Updates: Flynt release channel
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::GeneralUpdates {
                    SettingsSection { heading: "Updates",
                        SettingsRow {
                            label: "Channel",
                            hint: "Which Flynt release stream the update checker uses. Stable follows GitHub's latest production release. Nightly scans timestamped nightly prereleases and requires a signed nightly manifest.",
                            div { class: "radio-group",
                                for channel in UpdateChannel::all_named() {
                                    {
                                        let selected = *flynt_update_channel.read() == *channel;
                                        let channel_value = *channel;
                                        let is_nightly = matches!(channel, UpdateChannel::Nightly);
                                        let mut class = String::from("radio-btn");
                                        if selected { class.push_str(" active"); }
                                        rsx! {
                                            button {
                                                class: "{class}",
                                                onclick: move |_| *flynt_update_channel.write() = channel_value,
                                                "{channel.label()}"
                                                if is_nightly {
                                                    crate::components::HelpHint {
                                                        text: "Nightly updates are built from unreleased main-branch work. The app verifies the signed nightly manifest and installer checksum before opening the installer.".to_string()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ════════════════════════════════════════════════════════════
                // Project: Name + Location + Indexing + Visualization + Publication
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::Project {

                SettingsSection { heading: "Identity",
                    SettingsRow {
                        label: "Name",
                        hint: "Display name for this project. Shows up in the project picker, the tab bar title, and the agent context. Doesn't rename the folder on disk.",
                        input {
                            class: "input settings-input",
                            r#type: "text",
                            value: "{project_name}",
                            oninput: move |e| *project_name.write() = e.value(),
                        }
                    }
                    SettingsRow {
                        label: "Location",
                        hint: "Folder on disk where this project's notes, .flynt/ index, and config live. Change it by opening a different project from the welcome screen.",
                        span { class: "settings-path muted", "{ctx.project_root().display()}" }
                    }
                }

                SettingsSection { heading: "Indexing",
                    SettingsRow { label: "Write frontmatter",
                        label { class: "checkbox-label",
                            input {
                                r#type: "checkbox",
                                checked: *write_frontmatter.read(),
                                onchange: move |e| *write_frontmatter.write() = e.checked(),
                            }
                            "Write stable UUIDs into file frontmatter (project-wide default)"
                        }
                        span { class: "settings-hint muted", "Disable for code repos — then use scopes below to opt in specific directories" }
                    }
                    SettingsRow { label: "Track index snapshot",
                        label { class: "checkbox-label",
                            input {
                                r#type: "checkbox",
                                checked: *track_index_snapshot.read(),
                                onchange: move |e| *track_index_snapshot.write() = e.checked(),
                            }
                            "Write .flynt/index.snapshot.jsonl for portable metadata review"
                        }
                        span { class: "settings-hint muted", "The SQLite database stays local. Enable this only when the repo should carry a deterministic JSONL snapshot of Flynt's indexed metadata." }
                    }
                    SettingsRow { label: "Managed scopes",
                        IndexingScopesEditor { scopes: indexing_scopes }
                    }
                    SettingsRow { label: "Project registry",
                        div { class: "settings-inline-actions",
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| {
                                    let project_root = ctx.project_root();
                                    let project = ctx.project();
                                    let mut msg = registry_msg;
                                    spawn(async move {
                                        match tokio::task::spawn_blocking(move || {
                                            crate::project_registry_commands::refresh_snapshot_for_project(project_root, project)
                                        }).await {
                                            Ok(Ok(snapshot)) => {
                                                *msg.write() = Some(("ok", format!(
                                                    "Snapshot refreshed: {} docs, {} artifacts, {} tasks, {} specs, {} edges.",
                                                    snapshot.source_summary.document_count,
                                                    snapshot.source_summary.visual_artifact_count,
                                                    snapshot.source_summary.task_count,
                                                    snapshot.source_summary.spec_count,
                                                    snapshot.source_summary.edge_count,
                                                )));
                                            }
                                            Ok(Err(error)) => *msg.write() = Some(("err", format!("Refresh failed: {error}"))),
                                            Err(error) => *msg.write() = Some(("err", format!("Refresh task failed: {error}"))),
                                        }
                                    });
                                },
                                "Refresh snapshot"
                            }
                            button {
                                class: "btn btn-sm btn-ghost",
                                onclick: move |_| {
                                    let project_root = ctx.project_root();
                                    let mut msg = registry_msg;
                                    spawn(async move {
                                        match tokio::task::spawn_blocking(move || {
                                            crate::project_registry_commands::snapshot_summary(&project_root)
                                        }).await {
                                            Ok(Ok(summary)) => {
                                                crate::project_registry_commands::log_snapshot_summary(&summary);
                                                *msg.write() = Some(("ok", format!(
                                                    "Logged snapshot summary: {} docs, {} artifacts, {} tasks, {} specs, {} validation diagnostics.",
                                                    summary.document_count,
                                                    summary.visual_artifact_count,
                                                    summary.task_count,
                                                    summary.spec_count,
                                                    summary.validation_diagnostic_count,
                                                )));
                                            }
                                            Ok(Err(error)) => *msg.write() = Some(("err", format!("Dump failed: {error}"))),
                                            Err(error) => *msg.write() = Some(("err", format!("Dump task failed: {error}"))),
                                        }
                                    });
                                },
                                "Dump summary to log"
                            }
                        }
                        span { class: "settings-hint muted", ".flynt/registry/project-registry.snapshot.json — generated, portable, safe to delete/rebuild." }
                        if let Some((kind, msg)) = registry_msg.read().as_ref() {
                            div { class: "settings-status {kind}", "{msg}" }
                        }
                    }
                }

                SettingsSection { heading: "Visualization",
                    SettingsRow { label: "Excalidraw auto-export",
                        label { class: "checkbox-label",
                            input {
                                r#type: "checkbox",
                                checked: *excalidraw_auto_export.read(),
                                onchange: move |e| *excalidraw_auto_export.write() = e.checked(),
                            }
                            "Auto-export SVG when drawings are saved"
                        }
                    }
                    SettingsRow { label: "D2 auto-render",
                        label { class: "checkbox-label",
                            input {
                                r#type: "checkbox",
                                checked: *d2_auto_render.read(),
                                onchange: move |e| *d2_auto_render.write() = e.checked(),
                            }
                            "Auto-render D2 diagrams to SVG"
                        }
                    }
                    SettingsRow { label: "D2 theme",
                        input {
                            class: "input settings-input settings-input-sm",
                            r#type: "number",
                            value: "{d2_theme}",
                            placeholder: "200",
                            oninput: move |e| *d2_theme.write() = e.value(),
                        }
                        span { class: "settings-hint muted", "(200 = dark, 0 = default)" }
                    }
                    SettingsRow { label: "D2 layout",
                        div { class: "radio-group",
                            for (value, label) in [("elk", "ELK"), ("dagre", "Dagre"), ("tala", "TALA")] {
                                button {
                                    class: if d2_layout.read().as_str() == value { "radio-btn active" } else { "radio-btn" },
                                    onclick: move |_| *d2_layout.write() = value.to_string(),
                                    "{label}"
                                }
                            }
                        }
                    }
                    SettingsRow { label: "D2 binary",
                        input {
                            class: "input settings-input",
                            r#type: "text",
                            value: "{d2_bin}",
                            placeholder: "d2 (on PATH)",
                            oninput: move |e| *d2_bin.write() = e.value(),
                        }
                    }
                }

                SettingsSection { heading: "Publication",
                    PublicationRulesEditor {
                        default_visibility: publication_default_visibility,
                        rules: publication_rules,
                    }
                }

                } // end Project

                // ════════════════════════════════════════════════════════════
                // Omegon → Profile: agent profile + posture
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonProfile {
                    crate::components::omegon::OmegonSettingsSection {}
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Providers: AI provider credentials AND git-hosting
                // tokens (GitHub, GitLab, Forgejo). One credential store
                // because omegon's auth.json holds both — the agent uses AI
                // providers directly, and flynt's git sync + push pipeline
                // read forge tokens from the same place.
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonProviders {
                    div { class: "settings-providers-hint",
                        "Tokens for AI providers (Anthropic, OpenAI\u{2026}) and git hosting (GitHub, GitLab\u{2026}) live here. The agent uses the AI providers; flynt's Sync section reads the forge tokens for push/pull."
                    }
                    ProviderSettingsSection {}
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Extensions: installed extensions manager
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonExtensions {
                    crate::components::omegon::ExtensionManagerSection {}
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Armory: browse + install extensions from the
                // omegon registry. The `extensions_search` ACP call hits
                // the running omegon host, which talks to the registry.
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonArmory {
                    crate::components::omegon::ArmorySection {}
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Skills: enable/disable bundled + custom skills
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonSkills {
                    {
                        let omegon_ctx = ctx.omegon();
                        let current_skills = ctx.omegon().load_operator_settings().enabled_skills;
                        rsx! {
                            crate::components::omegon::SkillSettingsSection {
                                enabled_skills: current_skills,
                                on_change: move |updated: Vec<String>| {
                                    let omegon = omegon_ctx.clone();
                                    let mut settings = omegon.load_operator_settings();
                                    settings.enabled_skills = updated;
                                    let _ = omegon.save_operator_settings(&settings);
                                },
                                extensions_dir: ctx.omegon().extensions_dir.clone(),
                                skills_dir: ctx.omegon().home_dir.join("skills"),
                            }
                        }
                    }
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Daemon: background agent daemon config
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonDaemon {
                    DaemonSettingsSection { config: daemon_config }
                }

                // ════════════════════════════════════════════════════════════
                // Omegon → Runtime: channel, binary override, runtime paths
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::OmegonRuntime {
                    SettingsSection { heading: "Runtime",
                        DeploymentDiagnosticCard { diagnostic: deployment_diagnostic.clone() }
                        ArmorySkillsDiagnosticCard {
                            report: armory_report.clone(),
                            message: armory_install_msg.read().clone(),
                            custom_skill_id,
                            on_activate: move |skill_id: String| {
                                let mut manifest = load_deployment_for_settings(&ctx.omegon()).manifest;
                                if crate::omegon_activation::activate_skill(&mut manifest, &skill_id) {
                                    match crate::omegon_activation::save_manifest(&ctx.omegon(), &manifest) {
                                        Ok(()) => {
                                            armory_install_msg.set(Some(format!("Activated {skill_id} for this project")));
                                            armory_install_refresh += 1;
                                        }
                                        Err(error) => armory_install_msg.set(Some(format!("Activation failed: {error}"))),
                                    }
                                }
                            },
                            on_deactivate: move |skill_id: String| {
                                let mut manifest = load_deployment_for_settings(&ctx.omegon()).manifest;
                                if crate::omegon_activation::deactivate_skill(&mut manifest, &skill_id) {
                                    match crate::omegon_activation::save_manifest(&ctx.omegon(), &manifest) {
                                        Ok(()) => {
                                            armory_install_msg.set(Some(format!("Deactivated {skill_id} for this project")));
                                            armory_install_refresh += 1;
                                        }
                                        Err(error) => armory_install_msg.set(Some(format!("Deactivation failed: {error}"))),
                                    }
                                } else {
                                    armory_install_msg.set(Some(format!("{skill_id} is required by Flynt and cannot be deactivated here")));
                                }
                            },
                            on_install: move |_| {
                                let Some(src) = rfd::FileDialog::new()
                                    .set_title("Select Armory skill package")
                                    .pick_folder()
                                else { return; };
                                let omegon_home = ctx.omegon().home_dir.clone();
                                match crate::armory_install::install_user_skill_package(&src, &omegon_home) {
                                    Ok(installed) => {
                                        armory_install_msg.set(Some(format!(
                                            "Installed {} to {}",
                                            installed.id,
                                            installed.destination.display()
                                        )));
                                        armory_install_refresh += 1;
                                    }
                                    Err(error) => armory_install_msg.set(Some(format!("Install failed: {error}"))),
                                }
                            }
                        }
                        CliProbeDiagnosticCard { probe: cli_probe.clone() }
                        SettingsRow {
                            label: "Channel",
                            hint: "Which release stream flynt resolves to when no binary override is set. Stable = production builds. RC = release candidates, near-stable. Nightly = latest unreleased work, may break.",
                            div { class: "radio-group",
                                for ch in flynt_core::models::OmegonChannel::all_named() {
                                    {
                                        let ch_clone = ch.clone();
                                        let lbl = ch.label().to_string();
                                        let is_nightly = matches!(ch, flynt_core::models::OmegonChannel::Nightly);
                                        let mut class = String::from("radio-btn");
                                        if *omegon_channel.read() == *ch { class.push_str(" active"); }
                                        rsx! {
                                            button {
                                                class: "{class}",
                                                onclick: move |_| *omegon_channel.write() = ch_clone.clone(),
                                                "{lbl}"
                                                if is_nightly {
                                                    crate::components::HelpHint {
                                                        text: "Nightly builds can contain unreleased breaking changes. Use only if you're comfortable downgrading via `omegon switch` when something breaks.".to_string()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        SettingsRow {
                            label: "Binary path",
                            hint: "Explicit path to an omegon executable. Highest-priority override — wins over the OMEGON_BIN env var and the channel resolver. Leave blank to auto-detect from channel.",
                            input {
                                class: "input settings-input",
                                r#type: "text",
                                value: "{omegon_bin_override}",
                                placeholder: "Auto-detect from channel",
                                oninput: move |e| *omegon_bin_override.write() = e.value(),
                            }
                        }
                        SettingsRow {
                            label: "Runtime root",
                            hint: "Override for omegon's runtime directory (where sessions, secrets, and per-extension state live). Defaults to ~/.omegon — only set this if you're isolating workspaces.",
                            input {
                                class: "input settings-input",
                                r#type: "text",
                                value: "{omegon_runtime_root}",
                                placeholder: "optional absolute path",
                                oninput: move |e| *omegon_runtime_root.write() = e.value(),
                            }
                        }
                        SettingsRow {
                            label: "Memory DB",
                            hint: "Path to omegon's persistent memory database (its long-term knowledge store). Defaults to a file inside the runtime root. Override to share memory across workspaces or store it somewhere durable.",
                            input {
                                class: "input settings-input",
                                r#type: "text",
                                value: "{omegon_mind_db_path}",
                                placeholder: "optional absolute path",
                                oninput: move |e| *omegon_mind_db_path.write() = e.value(),
                            }
                        }
                    }
                }

                // ════════════════════════════════════════════════════════════
                // Advanced: Local paths, Config file editor
                // ════════════════════════════════════════════════════════════
                if *active_page.read() == SettingsPage::Advanced {

                div { class: "settings-advanced-warning",
                    span { class: "settings-advanced-warning-icon", "\u{26A0}" }
                    div {
                        div { class: "settings-advanced-warning-title", "Here be dragons" }
                        div { class: "settings-advanced-warning-body",
                            "These settings affect storage paths and raw config. Editing them incorrectly can break the project's index, lose recent state, or prevent flynt from launching. Snapshot the .flynt directory before changing anything you don't recognize."
                        }
                    }
                }

                SettingsSection { heading: "Local paths",
                    SettingsRow {
                        label: "State root",
                        hint: "Override for the per-project local-state directory. Defaults to the platform app-data directory keyed by project path; the opened content root is not mutated for local DB state. Useful only when isolating workspaces.",
                        input {
                            class: "input settings-input",
                            r#type: "text",
                            value: "{local_state_root}",
                            placeholder: "optional absolute path",
                            oninput: move |e| *local_state_root.write() = e.value(),
                        }
                    }
                    SettingsRow {
                        label: "Index DB",
                        hint: "Override for the SQLite full-text index database. Defaults to a file inside the state root. Move it to a fast SSD location if you have a huge project.",
                        input {
                            class: "input settings-input",
                            r#type: "text",
                            value: "{flynt_index_db_path}",
                            placeholder: "optional absolute path",
                            oninput: move |e| *flynt_index_db_path.write() = e.value(),
                        }
                    }
                    SettingsRow { label: "Styrene Identity",
                        input {
                            class: "input settings-input",
                            r#type: "text",
                            value: "{styrene_identity_profile}",
                            placeholder: "optional local identity profile",
                            oninput: move |e| *styrene_identity_profile.write() = e.value(),
                        }
                    }
                }

                SettingsSection { heading: "Config file",
                    div { class: "settings-row",
                        span { class: "settings-label", "config.toml" }
                        div { class: "settings-control",
                            button {
                                class: "btn btn-ghost",
                                onclick: {
                                    let cp = config_path.clone();
                                    move |_| {
                                        let v = *show_raw_config.read();
                                        if !v {
                                            *raw_config_text.write() = std::fs::read_to_string(&cp).unwrap_or_default();
                                        }
                                        *show_raw_config.write() = !v;
                                    }
                                },
                                if *show_raw_config.read() { "Close editor" } else { "Edit config.toml" }
                            }
                            span { class: "settings-hint muted", "Power user: edit the project config directly" }
                        }
                    }
                    if *show_raw_config.read() {
                        div { class: "raw-config-editor",
                            textarea {
                                class: "input raw-config-textarea",
                                value: "{raw_config_text}",
                                rows: "20",
                                spellcheck: "false",
                                oninput: move |e| *raw_config_text.write() = e.value(),
                            }
                            div { class: "raw-config-actions",
                                button {
                                    class: "btn btn-primary",
                                    onclick: {
                                        let cp = config_path.clone();
                                        move |_| {
                                            let text = raw_config_text.read().clone();
                                            match toml::from_str::<ProjectConfig>(&text) {
                                                Ok(_) => {
                                                    if let Err(e) = std::fs::write(&cp, &text) {
                                                        *raw_config_msg.write() = Some(("err", "Write failed — check permissions."));
                                                        tracing::error!("raw config write: {e}");
                                                    } else {
                                                        *raw_config_msg.write() = Some(("ok", "Config saved. Restart or re-open project to apply."));
                                                    }
                                                }
                                                Err(_) => {
                                                    *raw_config_msg.write() = Some(("err", "Invalid TOML — fix syntax before saving."));
                                                }
                                            }
                                        }
                                    },
                                    "Save config.toml"
                                }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: {
                                        let cp = config_path.clone();
                                        move |_| {
                                            *raw_config_text.write() = std::fs::read_to_string(&cp).unwrap_or_default();
                                            *raw_config_msg.write() = None;
                                        }
                                    },
                                    "Revert"
                                }
                                if let Some((kind, msg)) = *raw_config_msg.read() {
                                    span {
                                        class: if kind == "ok" { "save-msg ok" } else { "save-msg err" },
                                        "{msg}"
                                    }
                                }
                            }
                        }
                    }
                }

                } // end Advanced
                } // end live-Omegon session guard

                // ── Save bar ─────────────────────────────────────────────────
                div { class: "settings-save-bar",
                    button { class: "btn btn-primary", onclick: save, "Save changes" }
                    if *active_page.read() == SettingsPage::Project {
                        button { class: "btn btn-ghost", onclick: publish_preview, "Export local preview" }
                    }
                    if let Some((kind, msg)) = *save_msg.read() {
                        span {
                            class: if kind == "ok" { "save-msg ok" } else { "save-msg err" },
                            "{msg}"
                        }
                    }
                    if let Some((kind, msg)) = &*publish_msg.read() {
                        span {
                            class: if *kind == "ok" { "save-msg ok" } else { "save-msg err" },
                            "{msg}"
                        }
                    }
                }
            }
        }
    }
}

// ── Sub-components ────────────────────────────────────────────────────────────

fn path_from_input(raw: &str) -> Option<std::path::PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(trimmed))
    }
}

fn string_from_input(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[component]
fn LiveOmegonSessionRequired(page: SettingsPage) -> Element {
    rsx! {
        SettingsSection { heading: "Omegon session required",
            div { class: "settings-row",
                div { class: "settings-hint muted",
                    "{page.label()} uses live Omegon extension APIs. Start or reconnect the agent session before managing this page."
                }
            }
        }
    }
}

#[component]
fn SettingsSection(heading: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "settings-section",
            h2 { class: "settings-heading", "{heading}" }
            div { class: "settings-rows", {children} }
        }
    }
}

#[component]
fn SettingsRow(
    label: &'static str,
    children: Element,
    /// Optional one-line explanation surfaced via a hover `(?)` icon
    /// next to the label. Use it when the setting's meaning isn't
    /// self-evident from the label alone.
    #[props(default = "")]
    hint: &'static str,
) -> Element {
    rsx! {
        div { class: "settings-row",
            span { class: "settings-label",
                "{label}"
                if !hint.is_empty() {
                    crate::components::HelpHint { text: hint.to_string() }
                }
            }
            div { class: "settings-control", {children} }
        }
    }
}

#[component]
fn ThemeCard(entry: UiTheme, active: bool, on_select: EventHandler<String>) -> Element {
    let bg = entry
        .vars
        .get("--background")
        .map(String::as_str)
        .unwrap_or("#06080e");
    let surface = entry.vars.get("--card").map(String::as_str).unwrap_or(bg);
    let primary = entry
        .vars
        .get("--primary")
        .map(String::as_str)
        .unwrap_or("#2ab4c8");
    let text = entry
        .vars
        .get("--foreground")
        .map(String::as_str)
        .unwrap_or("#c4d8e4");
    let badge = if entry.builtin {
        "Built-in"
    } else {
        "Imported"
    };

    rsx! {
        button {
            class: if active { "theme-card active" } else { "theme-card" },
            onclick: move |_| on_select.call(entry.id.clone()),
            div {
                class: "theme-preview",
                style: "background:{bg}; border-color:{primary};",
                div {
                    class: "theme-preview-bar",
                    style: "background:{surface};",
                }
                div {
                    class: "theme-preview-dot",
                    style: "background:{primary};",
                }
                span {
                    class: "theme-preview-text",
                    style: "color:{text};",
                    "Aa"
                }
            }
            span { class: "theme-name", "{entry.name}" }
            span { class: "theme-kind", "{badge}" }
            if active {
                span { class: "theme-active-badge", "✓" }
            }
        }
    }
}

fn load_deployment_for_settings(omegon: &OmegonRuntimeContext) -> LoadedDeploymentManifest {
    match std::fs::read_to_string(&omegon.deployment_path) {
        Ok(content) => {
            match flynt_core::omegon_deployment::OmegonDeploymentManifest::from_toml(&content) {
                Ok(manifest) => LoadedDeploymentManifest::loaded(
                    flynt_core::omegon_deployment::merge_with_default_required_activation(manifest),
                ),
                Err(error) => LoadedDeploymentManifest {
                    manifest: flynt_core::omegon_deployment::OmegonDeploymentManifest::default(),
                    source: DeploymentManifestSource::Invalid {
                        error: error.to_string(),
                    },
                },
            }
        }
        Err(_) => LoadedDeploymentManifest {
            manifest: flynt_core::omegon_deployment::OmegonDeploymentManifest::default(),
            source: DeploymentManifestSource::MissingDefault,
        },
    }
}

#[component]
fn DeploymentDiagnosticCard(diagnostic: DeploymentDiagnostic) -> Element {
    let status = diagnostic.status;
    let class = format!("deployment-diagnostic {}", status.class());
    rsx! {
        div { class: "settings-row",
            span { class: "settings-label", "Flynt ACP deployment" }
            div { class: "settings-control",
                div { class: "{class}",
                    div { class: "deployment-diagnostic-head",
                        span { class: "deployment-diagnostic-status", "{status.label()}" }
                        span { class: "deployment-diagnostic-summary", "{diagnostic.summary}" }
                    }
                    if !diagnostic.details.is_empty() {
                        ul { class: "deployment-diagnostic-details",
                            for detail in diagnostic.details.iter() {
                                li { "{detail}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ArmorySkillsDiagnosticCard(
    report: crate::armory_resolution::ArmoryResolutionReport,
    message: Option<String>,
    custom_skill_id: Signal<String>,
    on_activate: EventHandler<String>,
    on_deactivate: EventHandler<String>,
    on_install: EventHandler<()>,
) -> Element {
    let missing = report.missing_required_skills();
    let status = if missing.is_empty() {
        "Ready"
    } else {
        "Warning"
    };
    let summary = if missing.is_empty() {
        "All required Flynt skills resolve from project overrides, user Armory, or bundled fallbacks.".to_string()
    } else {
        format!(
            "{} required Flynt skill(s) are not installed.",
            missing.len()
        )
    };
    let class = if missing.is_empty() {
        "deployment-diagnostic ok"
    } else {
        "deployment-diagnostic warning"
    };

    rsx! {
        div { class: "settings-row",
            span { class: "settings-label", "Armory skills" }
            div { class: "settings-control",
                div { class: "{class}",
                    div { class: "deployment-diagnostic-head",
                        span { class: "deployment-diagnostic-status", "{status}" }
                        span { class: "deployment-diagnostic-summary", "{summary}" }
                    }
                    ul { class: "deployment-diagnostic-details",
                        for skill in report.skills.iter() {
                            li {
                                strong { "{skill.name}" }
                                " — {skill.source.label()}"
                                if let Some(path) = skill.path.as_ref() {
                                    " ({path.display()})"
                                }
                                if !flynt_core::omegon_deployment::OmegonDeploymentManifest::default()
                                    .activation
                                    .skills
                                    .contains(&skill.name)
                                {
                                    button {
                                        class: "btn btn-ghost btn-xs inline-skill-action",
                                        onclick: {
                                            let skill_id = skill.name.clone();
                                            move |_| on_deactivate.call(skill_id.clone())
                                        },
                                        "Deactivate"
                                    }
                                }
                            }
                        }
                    }
                    if let Some(message) = message.as_ref() {
                        div { class: "deployment-diagnostic-summary", "{message}" }
                    }
                    div { class: "deployment-diagnostic-actions",
                        input {
                            class: "settings-input skill-activation-input",
                            placeholder: "skill-id to activate",
                            value: "{custom_skill_id.read()}",
                            oninput: move |event| custom_skill_id.set(event.value()),
                        }
                        button {
                            class: "btn btn-ghost btn-xs",
                            onclick: move |_| {
                                let skill_id = custom_skill_id.read().trim().to_string();
                                if !skill_id.is_empty() {
                                    on_activate.call(skill_id);
                                }
                            },
                            "Activate skill"
                        }
                        button {
                            class: "btn btn-ghost btn-xs",
                            onclick: move |_| on_install.call(()),
                            "Install skill package…"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CliProbeDiagnosticCard(probe: Option<crate::omegon_cli_probe::OmegonCliProbeResult>) -> Element {
    let (status, summary, details) = match probe {
        Some(probe) => {
            let status = match probe.status {
                crate::omegon_cli_probe::OmegonCliProbeStatus::Compatible => "Ready",
                crate::omegon_cli_probe::OmegonCliProbeStatus::Unknown => "Unknown",
                crate::omegon_cli_probe::OmegonCliProbeStatus::Incompatible => "Blocked",
            };
            let summary = match probe.status {
                crate::omegon_cli_probe::OmegonCliProbeStatus::Compatible => {
                    format!(
                        "Omegon CLI contract v{} is compatible.",
                        probe.expected_contract_version
                    )
                }
                crate::omegon_cli_probe::OmegonCliProbeStatus::Unknown => {
                    format!(
                        "Omegon CLI contract v{} could not be fully verified.",
                        probe.expected_contract_version
                    )
                }
                crate::omegon_cli_probe::OmegonCliProbeStatus::Incompatible => {
                    format!(
                        "Omegon CLI contract v{} is incompatible.",
                        probe.expected_contract_version
                    )
                }
            };
            let mut details = vec![format!("Binary: {}", probe.binary.display())];
            if let Some(version) = probe.version {
                details.push(format!("Version: {version}"));
            }
            details.extend(probe.details);
            (status.to_string(), summary, details)
        }
        None => (
            "Unknown".into(),
            "Omegon CLI compatibility probe has not completed yet.".into(),
            Vec::new(),
        ),
    };
    let class = format!("deployment-diagnostic {}", status.to_lowercase());
    rsx! {
        div { class: "settings-row",
            span { class: "settings-label", "Omegon CLI contract" }
            div { class: "settings-control",
                div { class: "{class}",
                    div { class: "deployment-diagnostic-head",
                        span { class: "deployment-diagnostic-status", "{status}" }
                        span { class: "deployment-diagnostic-summary", "{summary}" }
                    }
                    if !details.is_empty() {
                        ul { class: "deployment-diagnostic-details",
                            for detail in details.iter() {
                                li { "{detail}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SyncRadio(
    label: &'static str,
    active: bool,
    on_select: EventHandler<()>,
    /// Prerequisite status — drives the disabled state, the inline
    /// caution/error message, and the help-hint tooltip.
    #[props(default = crate::sync_prereq::SyncBackendStatus::Available)]
    status: crate::sync_prereq::SyncBackendStatus,
    /// Always-on description of what the backend does. The disabled
    /// reason is appended on hover when applicable.
    description: &'static str,
) -> Element {
    let blocked = status.is_blocked();
    let explanation = status.explanation().map(String::from);
    let tooltip_text = match explanation.as_deref() {
        Some(reason) => format!("{description}\n\n{reason}"),
        None => description.to_string(),
    };
    let class = match (active, &status) {
        (true, _) => "radio-btn active",
        (false, crate::sync_prereq::SyncBackendStatus::Blocked(_)) => "radio-btn disabled",
        (false, crate::sync_prereq::SyncBackendStatus::Warning(_)) => "radio-btn warning",
        _ => "radio-btn",
    };
    let dot_class = match (active, &status) {
        (true, _) => "radio-dot active",
        (false, crate::sync_prereq::SyncBackendStatus::Blocked(_)) => "radio-dot disabled",
        _ => "radio-dot",
    };
    rsx! {
        button {
            class: "{class}",
            disabled: blocked && !active,
            onclick: move |_| {
                if !blocked || active {
                    on_select.call(());
                }
            },
            div { class: "{dot_class}" }
            "{label}"
            crate::components::HelpHint { text: tooltip_text }
        }
    }
}
