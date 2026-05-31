use crate::bootstrap::AppContext;
use flynt_core::{
    models::{Document, DocumentId, Frontmatter},
    store::ProjectStore,
    visual_artifacts::{
        ArtifactActionKind, ArtifactActionRequest, VisualArtifactKind, VisualArtifactRef,
    },
};
use std::path::{Path, PathBuf};

pub fn open_visual_artifact(
    ctx: &AppContext,
    kind: VisualArtifactKind,
    source_path: &Path,
    wrapper_path: Option<&Path>,
    title: &str,
) -> Option<(DocumentId, String)> {
    let project = ctx.project();
    let open_path = wrapper_path.unwrap_or(source_path);
    if let Ok(Some(doc)) = project.store.get_document_by_path(open_path) {
        return Some((doc.id, doc.title));
    }

    let durable_open_path =
        ensure_visual_artifact_wrapper(&project, open_path, source_path, kind, title)
            .unwrap_or_else(|| open_path.to_path_buf());
    if let Ok(Some(doc)) = project.store.get_document_by_path(&durable_open_path) {
        return Some((doc.id, doc.title));
    }

    let content = std::fs::read_to_string(project.root.join(&durable_open_path))
        .ok()
        .or_else(|| source_content_for_virtual_document(&project.root, source_path, kind))?;
    let id = DocumentId::new();
    let now = chrono::Utc::now();
    let doc = Document {
        id: id.clone(),
        path: durable_open_path,
        title: title.to_string(),
        content,
        frontmatter: Frontmatter::default(),
        outgoing_links: Vec::new(),
        created_at: now,
        updated_at: now,
        entity: None,
    };
    project.store.save_document(&doc).ok()?;
    Some((id, title.to_string()))
}

pub fn execute_artifact_action(
    ctx: &AppContext,
    request: &ArtifactActionRequest,
) -> Option<(DocumentId, String)> {
    match request.action {
        ArtifactActionKind::Open => {
            let title = artifact_label(&request.target.source_path);
            let wrapper_path = match (request.target.kind, request.policy.prefer_wrapper) {
                (VisualArtifactKind::ExcalidrawDrawing | VisualArtifactKind::DesignBoard, true) => {
                    Some(request.target.source_path.with_extension("md"))
                }
                _ => None,
            };
            open_visual_artifact(
                ctx,
                request.target.kind,
                &request.target.source_path,
                wrapper_path.as_deref(),
                &title,
            )
        }
        ArtifactActionKind::RevealSource => open_visual_artifact(
            ctx,
            request.target.kind,
            &request.target.source_path,
            Some(&request.target.source_path),
            &artifact_label(&request.target.source_path),
        ),
        ArtifactActionKind::ShowDependencies
        | ArtifactActionKind::ShowConsumers
        | ArtifactActionKind::Render(_)
        | ArtifactActionKind::Inspect => None,
    }
}

pub fn open_visual_artifact_ref(
    ctx: &AppContext,
    artifact: &VisualArtifactRef,
) -> Option<(DocumentId, String)> {
    let title = artifact_label(&artifact.source_path);
    let wrapper_path = match artifact.kind {
        VisualArtifactKind::ExcalidrawDrawing | VisualArtifactKind::DesignBoard => {
            Some(artifact.source_path.with_extension("md"))
        }
        _ => None,
    };
    open_visual_artifact(
        ctx,
        artifact.kind,
        &artifact.source_path,
        wrapper_path.as_deref(),
        &title,
    )
}

fn ensure_visual_artifact_wrapper(
    project: &flynt_store::project::Project,
    open_path: &Path,
    source_path: &Path,
    kind: VisualArtifactKind,
    title: &str,
) -> Option<PathBuf> {
    if project.root.join(open_path).exists() {
        return Some(open_path.to_path_buf());
    }
    let wrapper_path = source_path.with_extension("md");
    let file_name = source_path.file_name()?.to_string_lossy();
    let escaped_title = title.replace('"', "\\\"");
    let (tag, embed_extension) = match kind {
        VisualArtifactKind::ExcalidrawDrawing => ("drawing", "excalidraw"),
        VisualArtifactKind::DesignBoard => ("design_board", "board"),
        _ => return None,
    };
    if source_path.extension().and_then(|ext| ext.to_str()) != Some(embed_extension) {
        return None;
    }
    let content =
        format!("+++\ntitle = \"{escaped_title}\"\ntags = [\"{tag}\"]\n+++\n\n![[{file_name}]]\n");
    project
        .save_document_content(&wrapper_path, &content)
        .ok()?;
    Some(wrapper_path)
}

fn source_content_for_virtual_document(
    project_root: &Path,
    source_path: &Path,
    kind: VisualArtifactKind,
) -> Option<String> {
    match kind {
        VisualArtifactKind::D2Diagram => {
            std::fs::read_to_string(project_root.join(source_path)).ok()
        }
        VisualArtifactKind::ExcalidrawDrawing | VisualArtifactKind::DesignBoard => source_path
            .file_name()
            .map(|file_name| format!("![[{}]]\n", file_name.to_string_lossy())),
        _ => None,
    }
}

fn artifact_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("artifact"))
        .to_string()
}
