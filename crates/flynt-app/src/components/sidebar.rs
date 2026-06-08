use crate::{
    bootstrap::{AppContext, OmegonRuntimeContext},
    state::{BookmarkRefresh, Route, TabState},
};
use dioxus::prelude::*;
use flynt_core::{
    models::{Bookmark, BookmarkTarget, Document, DocumentMeta},
    project_registry::ProjectRegistry,
    sidebar_projection::{ArtifactNavItem, SidebarProjection, TextFileNavItem},
    store::ProjectStore,
    visual_artifacts::{
        ArtifactActionRequest, RenderArtifact, RenderFormat, RenderStatus, VisualArtifactKind,
        VisualArtifactRef,
    },
};
use rfd::FileDialog;
use std::{collections::BTreeMap, path::PathBuf};

// ── Sidebar ───────────────────────────────────────────────────────────────────

#[component]
pub fn Sidebar(mut active_route: Signal<Route>) -> Element {
    let ctx = use_context::<AppContext>();
    let mut refresh = use_context_provider(|| Signal::new(0_u64));

    // Debounced project watcher — coalesces rapid-fire events (e.g., during
    // reindex of 1000+ files) into a single sidebar refresh after 500ms of quiet.
    let project_events = ctx.project_events();
    use_effect(move || {
        let mut rx = project_events.subscribe();
        spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_) => {
                        // Drain any queued events within the debounce window
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        while rx.try_recv().is_ok() {}
                        *refresh.write() += 1;
                    }
                    Err(_) => break,
                }
            }
        });
    });

    // Sidebar projection — live registry-derived state, not the persisted
    // Project Registry snapshot. This keeps the sidebar aligned with the
    // dual-lane product model: filesystem-like text files plus semantic
    // artifacts.
    let mut docs: Signal<Option<Vec<DocumentMeta>>> = use_signal(|| None);
    let mut projection: Signal<Option<SidebarProjection>> = use_signal(|| None);
    let mut projection_error: Signal<Option<String>> = use_signal(|| None);
    use_effect(move || {
        let _ = refresh();
        let project = ctx.project();
        let mut list = project.store.list_documents().unwrap_or_default();
        list.retain(|doc| {
            let path = doc.path.to_string_lossy();
            !path.starts_with("ai/delegations/")
                && !path.starts_with("ai/memory/")
                && !path.starts_with("references/comms/")
        });
        list.sort_by(|a, b| {
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then_with(|| a.path.cmp(&b.path))
        });
        *docs.write() = Some(list);

        match ProjectRegistry::discover(ctx.project_root(), project.store.as_ref()) {
            Ok(registry) => {
                projection_error.set(None);
                projection.set(Some(SidebarProjection::from_registry(&registry)));
            }
            Err(err) => {
                let message = err.to_string();
                tracing::warn!(%message, "sidebar projection registry discovery failed");
                projection_error.set(Some(message));
                projection.set(Some(SidebarProjection::default()));
            }
        }
    });

    let creating = use_signal(|| false);
    let new_name = use_signal(String::new);
    let create_err = use_signal(|| Option::<String>::None);
    let sidebar_lane = use_signal(|| SidebarLane::Files);

    rsx! {
        nav { class: "sidebar",
            // ── Project selector (compact) ──────────────────────
            ProjectSelector {}

            if *active_route.read() == Route::Design {
                crate::components::DesignPanel {
                    docs: docs.read().clone().unwrap_or_default(),
                    refresh,
                    active_route,
                }
            } else {
                // ── File tree ─────────────────────────────────────
                div { class: "file-tree",
                    if *creating.read() {
                        NewNoteInput {
                            new_name,
                            create_err,
                            creating,
                            refresh,
                            active_route,
                        }
                    }
                    match (projection.read().as_ref(), docs.read().as_ref()) {
                        (None, _) => rsx! { span { class: "tree-item muted", "Loading…" } },
                        (Some(projection), Some(docs)) => rsx! {
                            if let Some(error) = projection_error.read().clone() {
                                div { class: "tree-empty", "Sidebar registry error: {error}" }
                            }
                            DualLaneTree {
                                projection: projection.clone(),
                                docs: docs.clone(),
                                lane: *sidebar_lane.read(),
                                sidebar_lane,
                                creating,
                                new_name,
                                create_err,
                            }
                        },
                        _ => rsx! { span { class: "tree-item muted", "Loading…" } },
                    }
                }

                BookmarksPanel { active_route }
            }

        }
    }
}

#[component]
fn BookmarksPanel(mut active_route: Signal<Route>) -> Element {
    let ctx = use_context::<AppContext>();
    let refresh = use_context::<Signal<BookmarkRefresh>>();
    let mut bookmark_refresh = use_context::<Signal<BookmarkRefresh>>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut search_query = use_context::<Signal<String>>();
    let mut collapsed = use_signal(|| true);

    let mut bookmarks: Signal<Vec<Bookmark>> = use_signal(Vec::new);
    use_effect(move || {
        let _ = refresh.read().0;
        let project = ctx.project();
        let list = project
            .load_bookmarks()
            .map(|file| file.bookmarks)
            .unwrap_or_default();
        let is_empty = list.is_empty();
        bookmarks.set(list);
        if !is_empty && *collapsed.peek() {
            collapsed.set(false);
        }
    });

    let count = bookmarks.read().len();
    rsx! {
        section { class: "bookmarks-panel",
            button {
                class: "bookmarks-header",
                onclick: move |_| {
                    let next = !*collapsed.read();
                    collapsed.set(next);
                },
                span { class: "bookmarks-header-title", "Bookmarks" }
                span { class: "bookmarks-count", "{count}" }
                span { class: "bookmarks-chevron", if *collapsed.read() { "\u{25B8}" } else { "\u{25BE}" } }
            }
            if !*collapsed.read() {
                div { class: "bookmarks-list",
                    if bookmarks.read().is_empty() {
                        div { class: "bookmarks-empty", "No bookmarks yet" }
                    } else {
                        for bookmark in bookmarks.read().iter().cloned() {
                            {
                                let id = bookmark.id.clone();
                                let target_label = bookmark.target.label();
                                let target = bookmark.target.clone();
                                let title = bookmark.title.clone();
                                rsx! {
                                    div { key: "{id}", class: "bookmark-row-wrap",
                                        button {
                                            class: "bookmark-row",
                                            title: "{target_label}: {title}",
                                            onclick: move |_| {
                                                open_bookmark_target(
                                                    &ctx,
                                                    &target,
                                                    &title,
                                                    &mut tab_state,
                                                    &mut active_route,
                                                    &mut search_query,
                                                );
                                            },
                                            span { class: "bookmark-kind", "{target_icon(&bookmark.target)}" }
                                            span { class: "bookmark-main",
                                                span { class: "bookmark-title", "{bookmark.title}" }
                                                span { class: "bookmark-target muted", "{target_label}" }
                                            }
                                        }
                                        button {
                                            class: "bookmark-remove",
                                            title: "Remove bookmark",
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                let project = ctx.project();
                                                let remove_id = id.clone();
                                                spawn(async move {
                                                    let _ = tokio::task::spawn_blocking(move || {
                                                        project.remove_bookmark(&remove_id)
                                                    }).await;
                                                    let next = bookmark_refresh.read().0.wrapping_add(1);
                                                    bookmark_refresh.write().0 = next;
                                                });
                                            },
                                            "\u{00D7}"
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
}

fn target_icon(target: &BookmarkTarget) -> &'static str {
    match target {
        BookmarkTarget::Note { .. } => "\u{25C7}",
        BookmarkTarget::Heading { .. } => "#",
        BookmarkTarget::Search { .. } => "\u{2315}",
        BookmarkTarget::Graph { .. } => "\u{25CE}",
        BookmarkTarget::DesignBoard { .. } => "\u{25A1}",
        BookmarkTarget::Drawing { .. } => "\u{270E}",
    }
}

fn open_bookmark_target(
    ctx: &AppContext,
    target: &BookmarkTarget,
    title: &str,
    tab_state: &mut Signal<TabState>,
    active_route: &mut Signal<Route>,
    search_query: &mut Signal<String>,
) {
    match target {
        BookmarkTarget::Note { document_id, path }
        | BookmarkTarget::Heading {
            document_id, path, ..
        } => {
            let project = ctx.project();
            let doc: Option<Document> = document_id
                .as_ref()
                .and_then(|id| project.store.get_document(id).ok().flatten())
                .or_else(|| project.store.get_document_by_path(path).ok().flatten());
            if let Some(doc) = doc {
                tab_state.write().open(doc.id.clone(), doc.title.clone());
            } else {
                tracing::warn!("Bookmark target not found: {title}");
            }
            *active_route.write() = Route::Notes;
        }
        BookmarkTarget::Search { query } => {
            *search_query.write() = query.clone();
            *active_route.write() = Route::Search;
        }
        BookmarkTarget::Graph { .. } => {
            *active_route.write() = Route::Graph;
        }
        BookmarkTarget::DesignBoard { path } | BookmarkTarget::Drawing { path } => {
            let project = ctx.project();
            if let Ok(Some(doc)) = project.store.get_document_by_path(path) {
                tab_state.write().open(doc.id, doc.title);
            }
            *active_route.write() = Route::Notes;
        }
    }
}

// ── File tree builder ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarLane {
    Files,
    Surfaces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeSortMode {
    TitleAsc,
    TitleDesc,
    PathAsc,
    PathDesc,
    UpdatedNewest,
    UpdatedOldest,
}

impl TreeSortMode {
    fn label(self) -> &'static str {
        match self {
            Self::TitleAsc => "Title A→Z",
            Self::TitleDesc => "Title Z→A",
            Self::PathAsc => "Path A→Z",
            Self::PathDesc => "Path Z→A",
            Self::UpdatedNewest => "Updated newest",
            Self::UpdatedOldest => "Updated oldest",
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "title-desc" => Self::TitleDesc,
            "path-asc" => Self::PathAsc,
            "path-desc" => Self::PathDesc,
            "updated-newest" => Self::UpdatedNewest,
            "updated-oldest" => Self::UpdatedOldest,
            _ => Self::TitleAsc,
        }
    }

    fn value(self) -> &'static str {
        match self {
            Self::TitleAsc => "title-asc",
            Self::TitleDesc => "title-desc",
            Self::PathAsc => "path-asc",
            Self::PathDesc => "path-desc",
            Self::UpdatedNewest => "updated-newest",
            Self::UpdatedOldest => "updated-oldest",
        }
    }
}

const TREE_SORT_MODES: [TreeSortMode; 6] = [
    TreeSortMode::TitleAsc,
    TreeSortMode::TitleDesc,
    TreeSortMode::PathAsc,
    TreeSortMode::PathDesc,
    TreeSortMode::UpdatedNewest,
    TreeSortMode::UpdatedOldest,
];

#[component]
fn DualLaneTree(
    projection: SidebarProjection,
    docs: Vec<DocumentMeta>,
    lane: SidebarLane,
    mut sidebar_lane: Signal<SidebarLane>,
    mut creating: Signal<bool>,
    mut new_name: Signal<String>,
    mut create_err: Signal<Option<String>>,
) -> Element {
    let mut tree_sort = use_signal(|| TreeSortMode::TitleAsc);
    let mut hide_dotfiles = use_signal(|| true);
    let doc_by_id: BTreeMap<_, _> = docs
        .into_iter()
        .map(|doc| (doc.id.0.to_string(), doc))
        .collect();
    let current_sort = *tree_sort.read();
    rsx! {
        div { class: "sidebar-lane-switcher",
            button {
                class: if lane == SidebarLane::Files { "sidebar-lane-switch active" } else { "sidebar-lane-switch" },
                title: "Ordinary project text files",
                onclick: move |_| sidebar_lane.set(SidebarLane::Files),
                "Files"
            }
            button {
                class: if lane == SidebarLane::Surfaces { "sidebar-lane-switch active" } else { "sidebar-lane-switch" },
                title: "Visual surfaces: boards, drawings, diagrams, and flows",
                onclick: move |_| sidebar_lane.set(SidebarLane::Surfaces),
                "Surfaces"
            }
        }
        match lane {
            SidebarLane::Files => rsx! {
                section { class: "sidebar-lane sidebar-lane-notes",
                    div { class: "file-tree-section-title file-tree-section-title-row",
                        span { title: "Ordinary project text files", "Files" }
                        select {
                            class: "file-tree-sort-select",
                            title: "Sort files",
                            value: "{current_sort.value()}",
                            onchange: move |e: Event<FormData>| tree_sort.set(TreeSortMode::from_value(&e.value())),
                            for mode in TREE_SORT_MODES {
                                option {
                                    value: "{mode.value()}",
                                    selected: current_sort == mode,
                                    "{mode.label()}"
                                }
                            }
                        }
                        button {
                            class: if *hide_dotfiles.read() { "file-tree-filter-toggle active" } else { "file-tree-filter-toggle" },
                            title: if *hide_dotfiles.read() { "Show dot-prefixed files and folders" } else { "Hide dot-prefixed files and folders" },
                            onclick: move |_| {
                                let next = !*hide_dotfiles.peek();
                                hide_dotfiles.set(next);
                            },
                            ".hidden"
                        }
                        button {
                            class: "file-tree-new-btn file-tree-section-action",
                            title: "New file (⌘N)",
                            onclick: move |_| {
                                let was = *creating.read();
                                creating.set(!was);
                                if !was {
                                    new_name.set(String::new());
                                    create_err.set(None);
                                }
                            },
                            "+"
                        }
                    }
                    if projection.text_files.is_empty() {
                        div { class: "tree-empty", "No text files" }
                    } else {
                        { build_text_file_tree(&projection.text_files, &doc_by_id, current_sort, *hide_dotfiles.read()) }
                    }
                }
            },
            SidebarLane::Surfaces => rsx! {
                section { class: "sidebar-lane sidebar-lane-artifacts",
                    div { class: "file-tree-section-title", title: "Visual surfaces backed by source/wrapper/render files", "Surfaces" }
                    if !projection.artifacts.boards.is_empty() {
                        ArtifactGroup { label: "Boards", items: projection.artifacts.boards.clone() }
                    }
                    if !projection.artifacts.drawings.is_empty() {
                        ArtifactGroup { label: "Drawings", items: projection.artifacts.drawings.clone() }
                    }
                    if !projection.artifacts.diagrams.is_empty() {
                        ArtifactGroup { label: "Diagrams", items: projection.artifacts.diagrams.clone() }
                    }
                    if !projection.artifacts.flows.is_empty() {
                        ArtifactGroup { label: "Flows", items: projection.artifacts.flows.clone() }
                    }
                    if projection.artifacts.boards.is_empty() && projection.artifacts.drawings.is_empty() && projection.artifacts.diagrams.is_empty() && projection.artifacts.flows.is_empty() {
                        div { class: "tree-empty", "No surfaces" }
                    }
                }
            },
        }
    }
}

fn build_text_file_tree(
    items: &[TextFileNavItem],
    doc_by_id: &BTreeMap<String, DocumentMeta>,
    sort_mode: TreeSortMode,
    hide_dotfiles: bool,
) -> Element {
    let mut root: BTreeMap<String, TreeNode> = BTreeMap::new();
    let mut hidden_count = 0usize;
    for item in items {
        let Some(id) = &item.id else {
            continue;
        };
        let Some(doc) = doc_by_id.get(&id.0.to_string()) else {
            continue;
        };
        if hide_dotfiles && is_dot_prefixed_path(&doc.path) {
            hidden_count += 1;
            continue;
        }
        insert_document_tree_node(&mut root, doc, sort_mode);
    }
    if root.is_empty() {
        let message = if hidden_count > 0 {
            format!("No visible files ({hidden_count} dot-prefixed hidden)")
        } else {
            "No text files".to_string()
        };
        return rsx! { div { class: "tree-empty", "{message}" } };
    }

    rsx! { { render_tree_level(&root, 0, "") } }
}

fn is_dot_prefixed_path(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|part| part.starts_with('.') && part != ".")
    })
}

fn insert_document_tree_node(
    root: &mut BTreeMap<String, TreeNode>,
    doc: &DocumentMeta,
    sort_mode: TreeSortMode,
) {
    let components: Vec<_> = doc
        .path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if components.len() <= 1 {
        let filename = components
            .last()
            .cloned()
            .unwrap_or_else(|| doc.title.clone());
        root.entry(tree_file_sort_key(doc, &filename, sort_mode))
            .or_insert(TreeNode::File(doc.clone()));
        return;
    }
    let mut current = root;
    for part in &components[..components.len() - 1] {
        let entry = current
            .entry(part.clone())
            .or_insert_with(|| TreeNode::Folder {
                name: part.clone(),
                children: BTreeMap::new(),
                default_open: part == "notes",
            });
        current = match entry {
            TreeNode::Folder { children, .. } => children,
            _ => return,
        };
    }
    let filename = components
        .last()
        .cloned()
        .unwrap_or_else(|| doc.title.clone());
    current
        .entry(tree_file_sort_key(doc, &filename, sort_mode))
        .or_insert(TreeNode::File(doc.clone()));
}

fn tree_file_sort_key(doc: &DocumentMeta, filename: &str, sort_mode: TreeSortMode) -> String {
    let title = doc.title.to_lowercase();
    let filename = filename.to_lowercase();
    let path = doc.path.to_string_lossy().to_lowercase();
    let updated = doc.updated_at.timestamp_millis();
    match sort_mode {
        TreeSortMode::TitleAsc => format!("~{title}\u{0}{filename}"),
        TreeSortMode::TitleDesc => format!("~{}\u{0}{filename}", invert_sort_text(&title)),
        TreeSortMode::PathAsc => format!("~{path}\u{0}{title}"),
        TreeSortMode::PathDesc => format!("~{}\u{0}{title}", invert_sort_text(&path)),
        TreeSortMode::UpdatedNewest => format!("~{:020}\u{0}{title}", i64::MAX - updated),
        TreeSortMode::UpdatedOldest => format!("~{:020}\u{0}{title}", updated - i64::MIN),
    }
}

fn invert_sort_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| char::from_u32(char::MAX as u32 - ch as u32).unwrap_or(ch))
        .collect()
}

#[component]
fn ArtifactGroup(label: &'static str, items: Vec<ArtifactNavItem>) -> Element {
    let mut open = use_signal(|| true);
    let count = items.len();
    rsx! {
        button {
            class: "tree-item tree-folder",
            style: "padding-left: 8px;",
            onclick: move |_| { let v = *open.read(); *open.write() = !v; },
            span { class: "tree-chevron", if *open.read() { "\u{25BE}" } else { "\u{25B8}" } }
            span { class: "tree-name", "{label}" }
            span { class: "tree-count", "{count}" }
        }
        if *open.read() {
            for item in items {
                ArtifactNavFile { key: "{item.source_path.display()}", item: item.clone() }
            }
        }
    }
}

#[component]
fn ArtifactNavFile(item: ArtifactNavItem) -> Element {
    let kind = match item.kind {
        flynt_core::sidebar_projection::ArtifactNavKind::Board => VisualArtifactKind::DesignBoard,
        flynt_core::sidebar_projection::ArtifactNavKind::Drawing => {
            VisualArtifactKind::ExcalidrawDrawing
        }
        flynt_core::sidebar_projection::ArtifactNavKind::Diagram => VisualArtifactKind::D2Diagram,
        flynt_core::sidebar_projection::ArtifactNavKind::Flow => VisualArtifactKind::Flow,
    };
    let consumes = Vec::new();
    render_virtual_artifact_file(&item, kind, &consumes, 1)
}

/// Recursive tree node — folders contain sub-folders and files.
#[derive(Clone, PartialEq)]
enum TreeNode {
    Folder {
        name: String,
        children: BTreeMap<String, TreeNode>,
        default_open: bool,
    },
    File(DocumentMeta),
}

impl TreeNode {
    fn file_count(&self) -> usize {
        match self {
            Self::File(_) => 1,
            Self::Folder { children, .. } => children.values().map(|c| c.file_count()).sum(),
        }
    }

    fn contains_document_id(&self, id: &str) -> bool {
        match self {
            Self::File(meta) => meta.id.0.to_string() == id,
            Self::Folder { children, .. } => children
                .values()
                .any(|child| child.contains_document_id(id)),
        }
    }
}

/// Recursively render a tree level using keyed components for stable hook identity.
fn render_tree_level(nodes: &BTreeMap<String, TreeNode>, depth: u32, path_prefix: &str) -> Element {
    let entries: Vec<_> = nodes.iter().collect();

    rsx! {
        for (_key, node) in entries.iter() {
            match *node {
                TreeNode::Folder { name, children, default_open } => {
                    let full_path = if path_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{path_prefix}/{name}")
                    };
                    rsx! {
                        div { key: "{full_path}",
                            { render_folder_keyed(name, &full_path, children, depth, *default_open) }
                        }
                    }
                },
                TreeNode::File(doc) => {
                    let doc_key = doc.id.0.to_string();
                    rsx! {
                        TreeFile { key: "{doc_key}", meta: doc.clone(), depth }
                    }
                },
            }
        }
    }
}

fn render_virtual_artifact_file(
    item: &ArtifactNavItem,
    kind: VisualArtifactKind,
    consumes: &[VisualArtifactRef],
    depth: u32,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut active_route = use_context::<Signal<Route>>();
    let mut ctx_menu = use_signal(|| None::<(f64, f64)>);
    let title = artifact_display_title(item);
    let path = item.source_path.clone();
    let wrapper_path = item.wrapper_path.clone();
    let render_paths = item.render_paths.clone();
    let primary_format = primary_render_format(kind);
    let secondary_format = secondary_render_format(kind);
    let primary_status = render_status_for(&render_paths, primary_format);
    let secondary_status = render_status_for(&render_paths, secondary_format);
    let primary_format_label = render_format_label(primary_format);
    let secondary_format_label = render_format_label(secondary_format);
    let primary_format_short = primary_format.short_label();
    let artifact_title = visual_artifact_title(kind, &path, consumes);
    let click_path = path.clone();
    let menu_path = path.clone();
    let menu_wrapper_path = wrapper_path.clone();
    let menu_render_paths = render_paths.clone();
    let has_wrapper = wrapper_path.is_some();
    let has_outputs = render_paths
        .iter()
        .any(|render| render.status != RenderStatus::Missing);
    let d2_count = consumed_count(consumes, VisualArtifactKind::D2Diagram);
    let drawing_count = consumed_count(consumes, VisualArtifactKind::ExcalidrawDrawing);
    let indent = depth as f32 * 12.0;
    rsx! {
        button {
            class: "tree-item tree-file",
            style: "padding-left: {indent + 24.0}px;",
            title: "{artifact_title}",
            onclick: move |_| {
                open_artifact_action(&ctx, &mut tab_state, &mut active_route, kind, &click_path, ArtifactActionRequest::open);
            },
            oncontextmenu: move |e| {
                e.prevent_default();
                let coords = e.client_coordinates();
                *ctx_menu.write() = Some((coords.x, coords.y));
            },
            span { class: "tree-name", "{title}" }
            span { class: "diagram-artifact-badge {primary_status.class()}", title: "{primary_format_label} {primary_status.label()}; secondary {secondary_format_label} {secondary_status.label()}", "{primary_format_short}" }
            if d2_count > 0 {
                span { class: "diagram-artifact-badge present", title: "Consumes {d2_count} D2 diagram(s)", "◇{d2_count}" }
            }
            if drawing_count > 0 {
                span { class: "diagram-artifact-badge present", title: "Consumes {drawing_count} Excalidraw drawing(s)", "✎{drawing_count}" }
            }
        }
        if let Some((x, y)) = *ctx_menu.read() {
            crate::components::ContextMenu {
                x, y,
                items: artifact_context_menu_items(has_wrapper, has_outputs),
                on_close: move |_| *ctx_menu.write() = None,
                on_select: move |action: String| {
                    *ctx_menu.write() = None;
                    match action.as_str() {
                        "open" => open_artifact_action(&ctx, &mut tab_state, &mut active_route, kind, &menu_path, ArtifactActionRequest::open),
                        "reveal-source" => reveal_project_path(&ctx, &menu_path),
                        "reveal-wrapper" => {
                            if let Some(path) = &menu_wrapper_path {
                                reveal_project_path(&ctx, path);
                            }
                        }
                        "reveal-outputs" => reveal_render_outputs(&ctx, &menu_render_paths),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn artifact_display_title(item: &ArtifactNavItem) -> String {
    let source_stem = item
        .source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(item.title.as_str());
    item.title
        .strip_suffix(".board")
        .or_else(|| item.title.strip_suffix(".excalidraw"))
        .or_else(|| item.title.strip_suffix(".d2"))
        .or_else(|| item.title.strip_suffix(".flow"))
        .unwrap_or(source_stem)
        .to_string()
}

fn artifact_context_menu_items(
    has_wrapper: bool,
    has_outputs: bool,
) -> Vec<crate::components::ContextMenuItem> {
    let mut items = vec![
        crate::components::ContextMenuItem::new("open", "Open"),
        crate::components::ContextMenuItem::new("reveal-source", "Reveal Source").sep(),
    ];
    if has_wrapper {
        items.push(crate::components::ContextMenuItem::new(
            "reveal-wrapper",
            "Reveal Wrapper",
        ));
    }
    if has_outputs {
        items.push(crate::components::ContextMenuItem::new(
            "reveal-outputs",
            "Reveal Generated Outputs",
        ));
    }
    items
}

fn reveal_render_outputs(ctx: &AppContext, renders: &[RenderArtifact]) {
    if let Some(render) = renders
        .iter()
        .find(|render| render.status != RenderStatus::Missing)
    {
        reveal_project_path(ctx, &render.path);
    }
}

fn reveal_project_path(ctx: &AppContext, relative_path: &std::path::Path) {
    let abs = ctx.project().root.join(relative_path);
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(&abs)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(dir) = abs.parent() {
            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
        }
    }
}

fn open_artifact_action(
    ctx: &AppContext,
    tab_state: &mut Signal<TabState>,
    active_route: &mut Signal<Route>,
    kind: VisualArtifactKind,
    path: &std::path::Path,
    build: fn(VisualArtifactRef) -> ArtifactActionRequest,
) {
    let target = VisualArtifactRef {
        kind,
        source_path: path.to_path_buf(),
    };
    let request = build(target);
    if let Some((id, title)) = crate::visual_artifact_open::execute_artifact_action(ctx, &request) {
        tab_state.write().open(id, title);
        *active_route.write() = Route::Notes;
    }
}

fn visual_artifact_title(
    kind: VisualArtifactKind,
    path: &std::path::Path,
    consumes: &[VisualArtifactRef],
) -> String {
    let mut title = format!("{}: {}", visual_artifact_label(kind), path.display());
    if consumes.is_empty() {
        return title;
    }
    title.push_str("\nConsumes:");
    for item in consumes {
        title.push_str(&format!(
            "\n- {}: {}",
            visual_artifact_label(item.kind),
            item.source_path.display()
        ));
    }
    title
}

fn consumed_count(consumes: &[VisualArtifactRef], kind: VisualArtifactKind) -> usize {
    consumes.iter().filter(|item| item.kind == kind).count()
}

fn primary_render_format(kind: VisualArtifactKind) -> RenderFormat {
    match kind {
        VisualArtifactKind::DesignBoard => RenderFormat::Html,
        _ => RenderFormat::Svg,
    }
}

fn secondary_render_format(_kind: VisualArtifactKind) -> RenderFormat {
    RenderFormat::Png
}

fn visual_artifact_label(kind: VisualArtifactKind) -> &'static str {
    match kind {
        VisualArtifactKind::D2Diagram => "D2 diagram",
        VisualArtifactKind::ExcalidrawDrawing => "Excalidraw drawing",
        VisualArtifactKind::DesignBoard => "Design board",
        VisualArtifactKind::Flow => "Flow",
    }
}

fn render_status_for(renders: &[RenderArtifact], format: RenderFormat) -> RenderStatus {
    renders
        .iter()
        .find(|render| render.format == format)
        .map(|render| render.status)
        .unwrap_or(RenderStatus::Missing)
}

fn render_format_label(format: RenderFormat) -> &'static str {
    match format {
        RenderFormat::Svg => "SVG",
        RenderFormat::Png => "PNG",
        RenderFormat::Html => "HTML",
    }
}

impl RenderFormatLabel for RenderFormat {
    fn short_label(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Html => "html",
        }
    }
}

trait RenderFormatLabel {
    fn short_label(self) -> &'static str;
}

impl RenderStatusBadgeClass for RenderStatus {
    fn class(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Present => "present",
        }
    }
}

trait RenderStatusBadgeClass {
    fn class(self) -> &'static str;
}

/// Folder wrapper — uses a keyed div so Dioxus allocates a stable hook scope
/// per folder identity. The actual hook (`use_signal`) lives inside this
/// keyed scope and survives folder list changes.
fn render_folder_keyed(
    name: &str,
    path: &str,
    children: &BTreeMap<String, TreeNode>,
    depth: u32,
    default_open: bool,
) -> Element {
    let name = name.to_string();
    let path = path.to_string();
    let children = children.clone();
    let count: usize = children.values().map(|c| c.file_count()).sum();
    let tab_state = use_context::<Signal<TabState>>();
    let active_id = tab_state.read().active_id().map(|id| id.0.to_string());
    let should_open = active_id.as_deref().is_some_and(|id| {
        children
            .values()
            .any(|child| child.contains_document_id(id))
    });
    let mut open = use_signal(|| should_open || default_open);
    use_effect(move || {
        if should_open && !*open.read() {
            *open.write() = true;
        }
    });
    let indent = depth as f32 * 12.0;

    rsx! {
        button {
            class: "tree-item tree-folder",
            style: "padding-left: {indent + 8.0}px;",
            onclick: move |_| { let v = *open.read(); *open.write() = !v; },
            span { class: "tree-chevron", if *open.read() { "\u{25BE}" } else { "\u{25B8}" } }
            span { class: "tree-name", "{name}" }
            span { class: "tree-count", "{count}" }
        }
        if *open.read() {
            { render_tree_level(&children, depth + 1, &path) }
        }
    }
}

fn file_type_badge(path: &std::path::Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" => Some("txt"),
        "toml" => Some("toml"),
        "yaml" | "yml" => Some("yaml"),
        "json" => Some("json"),
        "csv" => Some("csv"),
        _ => None,
    }
}

#[component]
fn TreeFile(meta: DocumentMeta, depth: u32) -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut active_route = use_context::<Signal<Route>>();
    let mut refresh = use_context::<Signal<u64>>();
    let mut rename_trigger = use_context::<Signal<crate::state::RenameTrigger>>();

    let active_id = tab_state.read().active_id().cloned();
    let is_active = active_id.as_ref() == Some(&meta.id);

    let id = meta.id.clone();
    let title = meta.title.clone();
    let doc_path = meta.path.clone();
    let doc_title = meta.title.clone();
    let indent = depth as f32 * 12.0;

    let mut ctx_menu: Signal<Option<(f64, f64)>> = use_signal(|| None);

    // Task files live under Tasks/ — they're real notes but get a
    // subtle visual cue (different icon + class) so the operator can
    // pick them out in the tree without confusing them with regular
    // notes. Detection by path prefix because the .md frontmatter
    // already carries `kind = "task"` but isn't surfaced on
    // DocumentMeta yet.
    let is_task =
        meta.path.starts_with("Tasks/") || meta.path.to_string_lossy().starts_with("Tasks/");
    let file_badge = file_type_badge(&meta.path);

    rsx! {
        button {
            class: match (is_active, is_task) {
                (true, true)   => "tree-item tree-file tree-file-task note-preview-anchor active",
                (true, false)  => "tree-item tree-file note-preview-anchor active",
                (false, true)  => "tree-item tree-file tree-file-task note-preview-anchor",
                (false, false) => "tree-item tree-file note-preview-anchor",
            },
            style: "padding-left: {indent + 20.0}px;",
            onclick: move |_| {
                    if let Ok(Some(doc)) = ctx.project().store.get_document(&id) {
                        let _ = document::eval(&crate::views::notes::cm6_fast_swap_js(&doc.content));
                    }
                    tab_state.write().open(id.clone(), title.clone());
                    // Only write route if we're not already on Notes — avoids
                    // triggering a full app route re-evaluation for no reason.
                    if *active_route.peek() != Route::Notes {
                        *active_route.write() = Route::Notes;
                    }
            },
            oncontextmenu: move |e| {
                e.prevent_default();
                let coords = e.client_coordinates();
                *ctx_menu.write() = Some((coords.x, coords.y));
            },
            span { class: "tree-file-icon",
                if is_task { "\u{2611}" } else { "\u{25C7}" }
            }
            span { class: "tree-name", "{meta.title}" }
            if let Some(badge) = file_badge {
                span { class: "text-file-badge", "{badge}" }
            }
        }

        if let Some((x, y)) = *ctx_menu.read() {
            {
                let path_for_delete = doc_path.clone();
                let title_for_tab = doc_title.clone();
                let id_for_tab = meta.id.clone();
                let kind_items = {
                    use flynt_core::datum::EntityKind;
                    let current_kind = meta.entity_kind.clone();
                    let mut items = Vec::new();
                    if !matches!(current_kind, Some(EntityKind::DesignNode)) {
                        items.push(crate::components::ContextMenuItem::new("kind-design_node", "Convert to Design Node"));
                    }
                    if current_kind.is_some() {
                        items.push(crate::components::ContextMenuItem::new("kind-clear", "Remove Kind"));
                    }
                    if let Some(first) = items.first_mut() { *first = first.clone().sep(); }
                    items
                };
                rsx! {
                    crate::components::ContextMenu {
                        x, y,
                        items: {
                            let mut all = vec![
                                crate::components::ContextMenuItem::new("open-tab", "Open in New Tab"),
                                crate::components::ContextMenuItem::new("rename", "Rename\u{2026}"),
                                crate::components::ContextMenuItem::new("reveal", if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Open in File Manager" }),
                            ];
                            all.extend(kind_items);
                            all.push(crate::components::ContextMenuItem::danger("delete", "Delete").sep());
                            all
                        },
                        on_close: move |_| *ctx_menu.write() = None,
                        on_select: move |action: String| {
                            *ctx_menu.write() = None;
                            match action.as_str() {
                                "open-tab" => {
                                    if let Ok(Some(doc)) = ctx.project().store.get_document(&id_for_tab) {
                                        let _ = document::eval(&crate::views::notes::cm6_fast_swap_js(&doc.content));
                                    }
                                    tab_state.write().open(id_for_tab.clone(), title_for_tab.clone());
                                    *active_route.write() = Route::Notes;
                                }
                                "rename" => {
                                    tab_state.write().open(id_for_tab.clone(), title_for_tab.clone());
                                    *active_route.write() = Route::Notes;
                                    rename_trigger.write().0 += 1;
                                }
                                "reveal" => {
                                    let abs = ctx.project().root.join(&path_for_delete);
                                    #[cfg(target_os = "macos")]
                                    { let _ = std::process::Command::new("open").arg("-R").arg(&abs).spawn(); }
                                    #[cfg(target_os = "linux")]
                                    { if let Some(dir) = abs.parent() { let _ = std::process::Command::new("xdg-open").arg(dir).spawn(); } }
                                }
                                a if a.starts_with("kind-") => {
                                    let kind_val = &a[5..];
                                    let p = path_for_delete.clone();
                                    let kind_opt = if kind_val == "clear" { None } else { Some(kind_val.to_string()) };
                                    spawn(async move {
                                        let project = ctx.project();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            project.set_document_kind(&p, kind_opt.as_deref())
                                        }).await;
                                        *refresh.write() += 1;
                                    });
                                }
                                "delete" => {
                                    let p = path_for_delete.clone();
                                    let doc_id = id_for_tab.clone();
                                    spawn(async move {
                                        let project = ctx.project();
                                        let abs = project.root.join(&p);
                                        if abs.exists() {
                                            if let Ok(content) = std::fs::read_to_string(&abs) {
                                                if let Some(excalidraw_file) = crate::views::excalidraw::excalidraw_embed_path(&content) {
                                                    let doc_dir = p.parent().unwrap_or(std::path::Path::new(""));
                                                    let excalidraw_abs = project.root.join(doc_dir).join(&excalidraw_file);
                                                    let _ = std::fs::remove_file(&excalidraw_abs);
                                                }
                                            }
                                            let _ = std::fs::remove_file(&abs);
                                        }
                                        let _ = project.store.delete_document(&doc_id);
                                        let tabs = tab_state.read().tabs.clone();
                                        if let Some(idx) = tabs.iter().position(|(id, _)| id == &doc_id) {
                                            tab_state.write().close(idx);
                                        }
                                        let _ = project.reindex();
                                        *refresh.write() += 1;
                                    });
                                }
                                _ => {}
                            }
                        },
                    }
                }
            }
        }
    }
}

// ── New note input ────────────────────────────────────────────────────────────

#[component]
fn NewNoteInput(
    mut new_name: Signal<String>,
    mut create_err: Signal<Option<String>>,
    mut creating: Signal<bool>,
    mut refresh: Signal<u64>,
    mut active_route: Signal<Route>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();

    rsx! {
        div { class: "tree-new-note",
            input {
                class: "tree-new-note-input",
                placeholder: "path/name or name",
                value: "{new_name}",
                oninput: move |e| new_name.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Escape {
                        creating.set(false);
                        return;
                    }
                    if e.key() != Key::Enter { return; }
                    let raw = new_name.read().trim().to_string();
                    if raw.is_empty() { return; }
                    let rel = if raw.ends_with(".md") {
                        std::path::PathBuf::from(&raw)
                    } else {
                        std::path::PathBuf::from(format!("{raw}.md"))
                    };
                    let title = rel.file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| raw.clone());
                    let project = ctx.project();
                    let ctx2 = ctx.clone();
                    let title2 = title.clone();
                    spawn(async move {
                        match tokio::task::spawn_blocking(move || project.create_document(&rel, &title)).await {
                            Ok(Ok(())) => {
                                *refresh.write() += 1;
                                creating.set(false);
                                let project = ctx2.project();
                                if let Ok(Some(meta)) = tokio::task::spawn_blocking(
                                    move || project.store.find_document_by_slug(&title2)
                                ).await.unwrap_or(Ok(None)) {
                                    tab_state.write().open(meta.id, meta.title);
                                    *active_route.write() = Route::Notes;
                                }
                            }
                            Ok(Err(e)) => create_err.set(Some(e.to_string())),
                            Err(e) => create_err.set(Some(e.to_string())),
                        }
                    });
                },
                autofocus: true,
            }
            if let Some(ref err) = *create_err.read() {
                span { class: "tree-new-note-err", "{err}" }
            }
        }
    }
}

// ── Project selector ────────────────────────────────────────────────────────────

#[component]
fn ProjectSelector() -> Element {
    let mut ctx = use_context::<AppContext>();
    let mut active_route = use_context::<Signal<Route>>();
    let mut profile = use_signal(OmegonRuntimeContext::load_launcher_profile);
    let current_root = ctx.project_root();
    let current_name = ctx.project().config.project_name.clone();

    let mut do_switch = move |root: std::path::PathBuf| {
        let new_runtime = crate::bootstrap::runtime_state_for_project_root(root.clone());
        ctx.set_runtime(new_runtime);
        let mut updated = OmegonRuntimeContext::load_launcher_profile();
        updated.last_project_root = Some(root);
        let _ = OmegonRuntimeContext::save_launcher_profile(&updated);
        profile.set(updated);
        *active_route.write() = Route::Notes;
    };

    let open_folder = move |_| {
        let Some(selected_root) = FileDialog::new().pick_folder() else {
            return;
        };
        let name = selected_root
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Flynt")
            .to_string();
        if OmegonRuntimeContext::initialize_project(
            &selected_root,
            &name,
            flynt_core::models::SyncConfig::None,
        )
        .is_ok()
        {
            let mut updated = OmegonRuntimeContext::load_launcher_profile();
            OmegonRuntimeContext::register_known_project(&mut updated, &selected_root, &name);
            let _ = OmegonRuntimeContext::save_launcher_profile(&updated);
            profile.set(updated);
            do_switch(selected_root);
        }
    };

    let mut expanded = use_signal(|| false);
    let other_projects: Vec<_> = profile
        .read()
        .known_projects
        .iter()
        .filter(|v| v.root != current_root)
        .cloned()
        .collect();
    let has_others = !other_projects.is_empty();

    rsx! {
        div { class: "project-selector",
            button {
                class: "project-selector-btn",
                onclick: move |_| { let v = *expanded.read(); *expanded.write() = !v; },
                span { class: "project-selector-name", "{current_name}" }
                if has_others {
                    span { class: "project-selector-arrow",
                        if *expanded.read() { "\u{25BE}" } else { "\u{25B8}" }
                    }
                }
            }
            if *expanded.read() {
                div { class: "project-dropdown",
                    for project in other_projects {
                        {
                            let root = project.root.clone();
                            rsx! {
                                button {
                                    class: "project-dropdown-item",
                                    onclick: move |_| do_switch(root.clone()),
                                    "{project.name}"
                                }
                            }
                        }
                    }
                    button {
                        class: "project-dropdown-item muted",
                        onclick: open_folder,
                        "Open folder\u{2026}"
                    }
                }
            }
        }
    }
}

pub fn initial_note_id_for_project(project_root: &PathBuf) -> Option<String> {
    let project = crate::bootstrap::OmegonRuntimeContext::initialize_project(
        project_root,
        project_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Flynt"),
        flynt_core::models::SyncConfig::None,
    )
    .ok()?;
    project
        .store
        .list_documents()
        .ok()?
        .into_iter()
        .next()
        .map(|doc| doc.id.0.to_string())
}

#[cfg(test)]
mod sidebar_tests {
    use super::*;

    #[test]
    fn artifact_context_menu_gates_reveal_actions() {
        let source_only = artifact_context_menu_items(false, false)
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        assert_eq!(source_only, vec!["open", "reveal-source"]);

        let full = artifact_context_menu_items(true, true)
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        assert_eq!(
            full,
            vec!["open", "reveal-source", "reveal-wrapper", "reveal-outputs"]
        );
    }
}

#[cfg(test)]
mod sidebar_visual_tests {
    use super::*;
    use flynt_core::project_registry::VisualArtifactId;

    #[test]
    fn artifact_display_title_strips_source_extensions() {
        let item = ArtifactNavItem {
            id: VisualArtifactId("board:boards/Demo.board".into()),
            title: "Demo.board".into(),
            kind: flynt_core::sidebar_projection::ArtifactNavKind::Board,
            source_path: std::path::PathBuf::from("boards/Demo.board"),
            wrapper_path: None,
            render_paths: Vec::new(),
        };
        assert_eq!(artifact_display_title(&item), "Demo");
    }
}

#[cfg(test)]
mod sidebar_file_badge_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn dot_prefixed_path_detection_checks_any_component() {
        assert!(is_dot_prefixed_path(Path::new(".omegon/agent-journal.md")));
        assert!(is_dot_prefixed_path(Path::new("docs/.draft/note.md")));
        assert!(!is_dot_prefixed_path(Path::new("docs/regular.md")));
    }

    #[test]
    fn file_type_badge_marks_non_markdown_text_files() {
        assert_eq!(file_type_badge(std::path::Path::new("note.md")), None);
        assert_eq!(
            file_type_badge(std::path::Path::new("scratchpad.txt")),
            Some("txt")
        );
        assert_eq!(
            file_type_badge(std::path::Path::new("fixture.json")),
            Some("json")
        );
        assert_eq!(
            file_type_badge(std::path::Path::new("data.csv")),
            Some("csv")
        );
    }
}
