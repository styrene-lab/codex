use crate::report::{DiagnosticSeverity, ReportDiagnostic};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentBuildRequest {
    pub source: PathBuf,
    pub output_dir: PathBuf,
    pub preview: bool,
    pub pdf: bool,
    pub force: bool,
    pub inputs: Vec<TypstInput>,
    pub package_lock: Option<TypstPackageLock>,
    pub plugin_approvals: Option<TypstPluginApprovals>,
    pub world: TypstWorldPolicy,
}

impl FormalDocumentBuildRequest {
    pub fn new(source: impl Into<PathBuf>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            output_dir: output_dir.into(),
            preview: true,
            pdf: false,
            force: false,
            inputs: Vec::new(),
            package_lock: None,
            plugin_approvals: None,
            world: TypstWorldPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstWorldPolicy {
    pub package_mode: TypstPackageMode,
    pub font_mode: TypstFontMode,
    pub plugin_mode: TypstPluginMode,
    pub engine_preference: TypstEnginePreference,
    pub project_root: PathBuf,
    pub package_path: PathBuf,
    pub package_cache_path: PathBuf,
    pub font_paths: Vec<PathBuf>,
    pub creation_timestamp: Option<i64>,
}

impl Default for TypstWorldPolicy {
    fn default() -> Self {
        Self {
            package_mode: TypstPackageMode::AskBeforeDownload,
            font_mode: TypstFontMode::BundledAndProject,
            plugin_mode: TypstPluginMode::AskBeforeFirstHash,
            engine_preference: TypstEnginePreference::Bundled,
            project_root: PathBuf::new(),
            package_path: PathBuf::from(".flynt/typst/packages"),
            package_cache_path: PathBuf::from(".flynt/cache/typst/packages"),
            font_paths: vec![
                PathBuf::from("fonts"),
                PathBuf::from("typst/fonts"),
                PathBuf::from(".flynt/fonts"),
            ],
            creation_timestamp: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstPackageMode {
    OfflineOnly,
    AskBeforeDownload,
    AutoDownload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstFontMode {
    BundledAndProject,
    BundledProjectAndSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstPluginMode {
    DenyAll,
    AskBeforeFirstHash,
    AllowApproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstEnginePreference {
    Bundled,
    Embedded,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentBuildManifest {
    pub kind: String,
    pub source: PathBuf,
    pub source_sha256: String,
    pub built_at: DateTime<Utc>,
    pub engine: TypstEngineInfo,
    pub world: TypstWorldPolicySummary,
    pub outputs: FormalDocumentOutputs,
    pub diagnostics: Vec<ReportDiagnostic>,
    pub packages: Vec<TypstPackageUse>,
    pub fonts: Vec<TypstFontUse>,
    pub plugins: Vec<TypstPluginUse>,
    pub inputs: Vec<TypstInput>,
    pub assets: Vec<PathBuf>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstWorldPolicySummary {
    pub package_mode: TypstPackageMode,
    pub font_mode: TypstFontMode,
    pub plugin_mode: TypstPluginMode,
    pub project_root: PathBuf,
    pub package_path: PathBuf,
    pub package_cache_path: PathBuf,
    pub font_paths: Vec<PathBuf>,
}

impl From<&TypstWorldPolicy> for TypstWorldPolicySummary {
    fn from(policy: &TypstWorldPolicy) -> Self {
        Self {
            package_mode: policy.package_mode,
            font_mode: policy.font_mode,
            plugin_mode: policy.plugin_mode,
            project_root: policy.project_root.clone(),
            package_path: policy.package_path.clone(),
            package_cache_path: policy.package_cache_path.clone(),
            font_paths: policy.font_paths.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstEngineInfo {
    pub kind: TypstEnginePreference,
    pub version: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentOutputs {
    pub manifest: PathBuf,
    pub preview: Vec<PathBuf>,
    pub pdf: Option<PathBuf>,
    pub last_successful_preview: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPackageUse {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstFontUse {
    pub family: String,
    pub source: TypstFontSource,
    pub path: Option<PathBuf>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstFontSource {
    Bundled,
    Project,
    System,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPluginUse {
    pub path: PathBuf,
    pub sha256: String,
    pub approved: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentSettings {
    pub engine_mode: FormalDocumentEngineMode,
    pub package_mode: TypstPackageMode,
    pub plugin_mode: TypstPluginMode,
    pub font_mode: TypstFontMode,
    pub network_mode: TypstNetworkMode,
    pub build_pdf_by_default: bool,
    pub output_root: PathBuf,
    pub creation_timestamp: Option<i64>,
}

impl Default for FormalDocumentSettings {
    fn default() -> Self {
        Self {
            engine_mode: FormalDocumentEngineMode::Bundled,
            package_mode: TypstPackageMode::AskBeforeDownload,
            plugin_mode: TypstPluginMode::AskBeforeFirstHash,
            font_mode: TypstFontMode::BundledAndProject,
            network_mode: TypstNetworkMode::NeverWithoutApproval,
            build_pdf_by_default: false,
            output_root: PathBuf::from("reports"),
            creation_timestamp: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormalDocumentEngineMode {
    Bundled,
    System,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypstNetworkMode {
    NeverWithoutApproval,
    AllowPackageDownloads,
    OfflineOnly,
}

impl FormalDocumentSettings {
    pub fn world_policy(&self, project_root: impl Into<PathBuf>) -> TypstWorldPolicy {
        let project_root = project_root.into();
        TypstWorldPolicy {
            project_root: project_root.clone(),
            package_mode: match self.network_mode {
                TypstNetworkMode::OfflineOnly => TypstPackageMode::OfflineOnly,
                TypstNetworkMode::NeverWithoutApproval => self.package_mode,
                TypstNetworkMode::AllowPackageDownloads => self.package_mode,
            },
            plugin_mode: self.plugin_mode,
            font_mode: self.font_mode,
            engine_preference: match self.engine_mode {
                FormalDocumentEngineMode::Bundled => TypstEnginePreference::Bundled,
                FormalDocumentEngineMode::System => TypstEnginePreference::System,
                FormalDocumentEngineMode::Disabled => TypstEnginePreference::Bundled,
            },
            package_path: project_root.join(".flynt/typst/packages"),
            package_cache_path: project_root.join(".flynt/cache/typst/packages"),
            font_paths: vec![
                project_root.join("fonts"),
                project_root.join("typst/fonts"),
                project_root.join(".flynt/fonts"),
            ],
            creation_timestamp: self.creation_timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentDoctorReport {
    pub settings: FormalDocumentSettings,
    pub engine: DoctorCheck,
    pub package_path: DoctorCheck,
    pub package_cache_path: DoctorCheck,
    pub font_paths: Vec<DoctorCheck>,
    pub plugin_approval_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub ok: bool,
    pub label: String,
    pub detail: String,
}

pub fn formal_document_doctor(
    settings: FormalDocumentSettings,
    project_root: impl Into<PathBuf>,
    bundled_typst_path: Option<PathBuf>,
    plugin_approvals: Option<&TypstPluginApprovals>,
) -> FormalDocumentDoctorReport {
    let project_root = project_root.into();
    let world = settings.world_policy(&project_root);
    let engine = if settings.engine_mode == FormalDocumentEngineMode::Disabled {
        DoctorCheck {
            ok: false,
            label: "engine".to_string(),
            detail: "Formal Document engine disabled".to_string(),
        }
    } else {
        let locator = TypstEngineLocator {
            bundled_path: bundled_typst_path,
            allow_system: settings.engine_mode == FormalDocumentEngineMode::System,
        };
        match locator.resolve() {
            Ok(engine) => DoctorCheck {
                ok: true,
                label: "engine".to_string(),
                detail: format!(
                    "{:?} {}",
                    engine.engine_info().kind,
                    engine.engine_info().version
                ),
            },
            Err(error) => DoctorCheck {
                ok: false,
                label: "engine".to_string(),
                detail: error.to_string(),
            },
        }
    };
    FormalDocumentDoctorReport {
        settings,
        engine,
        package_path: DoctorCheck {
            ok: world.package_path.exists(),
            label: "package_path".to_string(),
            detail: world.package_path.display().to_string(),
        },
        package_cache_path: DoctorCheck {
            ok: world.package_cache_path.exists(),
            label: "package_cache_path".to_string(),
            detail: world.package_cache_path.display().to_string(),
        },
        font_paths: world
            .font_paths
            .iter()
            .map(|path| DoctorCheck {
                ok: path.exists(),
                label: "font_path".to_string(),
                detail: path.display().to_string(),
            })
            .collect(),
        plugin_approval_count: plugin_approvals
            .map(|approvals| approvals.approvals.len())
            .unwrap_or(0),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPackageLock {
    pub version: u32,
    pub packages: Vec<TypstPackageLockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPackageLockEntry {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPluginApprovals {
    pub version: u32,
    pub approvals: Vec<TypstPluginApproval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPluginApproval {
    pub sha256: String,
    pub source: String,
    pub path: PathBuf,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypstPolicyPreflight {
    pub diagnostics: Vec<ReportDiagnostic>,
    pub packages: Vec<TypstPackageUse>,
    pub plugins: Vec<TypstPluginUse>,
}

impl TypstPluginApprovals {
    fn approves(&self, sha256: &str) -> bool {
        self.approvals
            .iter()
            .any(|approval| approval.sha256 == sha256)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypstPackageImport {
    namespace: String,
    name: String,
    version: String,
}

pub fn preflight_typst_policy(
    source: &Path,
    world: &TypstWorldPolicy,
    package_lock: Option<&TypstPackageLock>,
    plugin_approvals: Option<&TypstPluginApprovals>,
) -> Result<TypstPolicyPreflight> {
    let body = fs::read_to_string(source)
        .with_context(|| format!("read Typst source {}", source.display()))?;
    let imports = scan_typst_package_imports(&body);
    let mut diagnostics = Vec::new();
    let mut packages = Vec::new();

    for import in imports {
        let package_dir = world
            .package_path
            .join(&import.namespace)
            .join(&import.name)
            .join(&import.version);
        if package_dir.exists() {
            packages.push(TypstPackageUse {
                namespace: import.namespace,
                name: import.name,
                version: import.version,
                source: "project-local".to_string(),
                sha256: Some(hash_directory(&package_dir)?),
            });
            continue;
        }
        let locked = package_lock.is_some_and(|lock| {
            lock.packages.iter().any(|entry| {
                entry.namespace == import.namespace
                    && entry.name == import.name
                    && entry.version == import.version
            })
        });
        if !locked {
            diagnostics.push(ReportDiagnostic {
                severity: match world.package_mode {
                    TypstPackageMode::AutoDownload => DiagnosticSeverity::Info,
                    _ => DiagnosticSeverity::Error,
                },
                code: "typst_package_missing".to_string(),
                message: format!(
                    "Typst package @{} / {}:{} is not available in the Flynt package path",
                    import.namespace, import.name, import.version
                ),
                span: None,
            });
        }
    }

    let mut plugins = Vec::new();
    for plugin_path in collect_wasm_files(&world.package_path)? {
        let sha256 = hash_file(&plugin_path)?;
        let approved = plugin_approvals.is_some_and(|approvals| approvals.approves(&sha256));
        match world.plugin_mode {
            TypstPluginMode::DenyAll => diagnostics.push(ReportDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "typst_plugin_denied".to_string(),
                message: format!("Typst plugin denied by policy: {}", plugin_path.display()),
                span: None,
            }),
            TypstPluginMode::AskBeforeFirstHash | TypstPluginMode::AllowApproved if !approved => {
                diagnostics.push(ReportDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    code: "typst_plugin_unapproved".to_string(),
                    message: format!(
                        "Typst plugin requires project approval: {} ({sha256})",
                        plugin_path.display()
                    ),
                    span: None,
                });
            }
            _ => {}
        }
        plugins.push(TypstPluginUse {
            path: plugin_path,
            sha256,
            approved,
            source: "package-path".to_string(),
        });
    }

    Ok(TypstPolicyPreflight {
        diagnostics,
        packages,
        plugins,
    })
}

fn scan_typst_package_imports(body: &str) -> Vec<TypstPackageImport> {
    let mut imports = Vec::new();
    for (start, _) in body.match_indices("@") {
        let rest = &body[start + 1..];
        let Some((namespace, rest)) = rest.split_once('/') else {
            continue;
        };
        let Some((name, rest)) = rest.split_once(':') else {
            continue;
        };
        let version: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
            .collect();
        if !namespace.is_empty() && !name.is_empty() && !version.is_empty() {
            imports.push(TypstPackageImport {
                namespace: namespace.to_string(),
                name: name.to_string(),
                version,
            });
        }
    }
    imports.sort_by(|a, b| {
        (&a.namespace, &a.name, &a.version).cmp(&(&b.namespace, &b.name, &b.version))
    });
    imports.dedup();
    imports
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

pub fn hash_directory(path: &Path) -> Result<String> {
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let rel = file.strip_prefix(path).unwrap_or(&file).to_string_lossy();
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(&file)?);
        hasher.update([0]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_wasm_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    Ok(files
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wasm"))
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormalDocumentBuildState {
    Missing,
    Clean,
    Dirty,
    Queued,
    Building,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalDocumentBuildResult {
    pub state: FormalDocumentBuildState,
    pub manifest: FormalDocumentBuildManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstEngineOutput {
    pub diagnostics: Vec<ReportDiagnostic>,
    pub preview: Vec<PathBuf>,
    pub pdf: Option<PathBuf>,
    pub assets: Vec<PathBuf>,
    pub packages: Vec<TypstPackageUse>,
    pub fonts: Vec<TypstFontUse>,
    pub plugins: Vec<TypstPluginUse>,
}

impl TypstEngineOutput {
    fn empty() -> Self {
        Self {
            diagnostics: Vec::new(),
            preview: Vec::new(),
            pdf: None,
            assets: Vec::new(),
            packages: Vec::new(),
            fonts: Vec::new(),
            plugins: Vec::new(),
        }
    }
}

pub trait TypstEngine {
    fn engine_info(&self) -> TypstEngineInfo;
    fn compile(&self, request: &FormalDocumentBuildRequest) -> Result<TypstEngineOutput>;
}

pub struct FormalDocumentBuildService<E: TypstEngine> {
    engine: E,
}

impl<E: TypstEngine> FormalDocumentBuildService<E> {
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    pub fn build(&self, request: &FormalDocumentBuildRequest) -> Result<FormalDocumentBuildResult> {
        validate_build_request(request)?;
        let started = std::time::Instant::now();
        fs::create_dir_all(&request.output_dir)
            .with_context(|| format!("create output dir {}", request.output_dir.display()))?;
        let raw = fs::read(&request.source)
            .with_context(|| format!("read formal document {}", request.source.display()))?;
        let previous_preview = previous_successful_preview(&request.output_dir);
        let policy_preflight = preflight_typst_policy(
            &request.source,
            &request.world,
            request.package_lock.as_ref(),
            request.plugin_approvals.as_ref(),
        )?;
        if policy_preflight
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        {
            let manifest = self.write_manifest(
                request,
                &raw,
                TypstEngineOutput {
                    diagnostics: policy_preflight.diagnostics,
                    preview: Vec::new(),
                    pdf: None,
                    assets: Vec::new(),
                    packages: policy_preflight.packages,
                    fonts: Vec::new(),
                    plugins: policy_preflight.plugins,
                },
                previous_preview,
                started.elapsed(),
            )?;
            return Ok(FormalDocumentBuildResult {
                state: FormalDocumentBuildState::Failed,
                manifest,
            });
        }
        let mut output = match self.engine.compile(request) {
            Ok(output) => output,
            Err(error) => {
                let manifest = self.write_manifest(
                    request,
                    &raw,
                    TypstEngineOutput {
                        diagnostics: vec![ReportDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            code: "formal_document_build_failed".to_string(),
                            message: error.to_string(),
                            span: None,
                        }],
                        preview: Vec::new(),
                        pdf: None,
                        assets: Vec::new(),
                        packages: Vec::new(),
                        fonts: Vec::new(),
                        plugins: Vec::new(),
                    },
                    previous_preview,
                    started.elapsed(),
                )?;
                return Ok(FormalDocumentBuildResult {
                    state: FormalDocumentBuildState::Failed,
                    manifest,
                });
            }
        };
        output.diagnostics.extend(policy_preflight.diagnostics);
        output.packages.extend(policy_preflight.packages);
        output.plugins.extend(policy_preflight.plugins);
        dedup_packages(&mut output.packages);
        dedup_plugins(&mut output.plugins);
        let failed = output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
        if failed {
            output.preview.clear();
            output.pdf = None;
        }
        let manifest =
            self.write_manifest(request, &raw, output, previous_preview, started.elapsed())?;
        let state = if manifest
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        {
            FormalDocumentBuildState::Failed
        } else {
            FormalDocumentBuildState::Succeeded
        };
        Ok(FormalDocumentBuildResult { state, manifest })
    }

    fn write_manifest(
        &self,
        request: &FormalDocumentBuildRequest,
        raw: &[u8],
        output: TypstEngineOutput,
        previous_preview: Vec<PathBuf>,
        elapsed: Duration,
    ) -> Result<FormalDocumentBuildManifest> {
        let manifest_path = request.output_dir.join("manifest.json");
        let failed = output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
        let manifest = FormalDocumentBuildManifest {
            kind: "formal_document_build".to_string(),
            source: request.source.clone(),
            source_sha256: sha256_hex(raw),
            built_at: Utc::now(),
            engine: self.engine.engine_info(),
            world: TypstWorldPolicySummary::from(&request.world),
            outputs: FormalDocumentOutputs {
                manifest: manifest_path.clone(),
                preview: if failed { Vec::new() } else { output.preview },
                pdf: if failed { None } else { output.pdf },
                last_successful_preview: if failed { previous_preview } else { Vec::new() },
            },
            diagnostics: output.diagnostics,
            packages: output.packages,
            fonts: output.fonts,
            plugins: output.plugins,
            inputs: request.inputs.clone(),
            assets: output.assets,
            duration_ms: duration_ms(elapsed),
        };
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
            .with_context(|| format!("write manifest {}", manifest_path.display()))?;
        Ok(manifest)
    }
}

/// Deterministic scaffold engine for Flynt's Formal Document contract tests.
pub struct StubTypstEngine {
    info: TypstEngineInfo,
}

impl StubTypstEngine {
    pub fn bundled_for_tests(version: impl Into<String>) -> Self {
        Self {
            info: TypstEngineInfo {
                kind: TypstEnginePreference::Bundled,
                version: version.into(),
                path: None,
            },
        }
    }
}

impl TypstEngine for StubTypstEngine {
    fn engine_info(&self) -> TypstEngineInfo {
        self.info.clone()
    }

    fn compile(&self, request: &FormalDocumentBuildRequest) -> Result<TypstEngineOutput> {
        let raw = fs::read(&request.source)?;
        let mut output = TypstEngineOutput::empty();
        let preview_dir = request.output_dir.join("preview");
        if request.preview {
            fs::create_dir_all(&preview_dir)?;
            let preview_path = preview_dir.join("page-01-of-01.svg");
            fs::write(&preview_path, stub_svg(&request.source))?;
            output.preview.push(preview_path);
        }
        if request.pdf {
            let path = request.output_dir.join("document.pdf");
            fs::write(&path, b"%PDF-1.7\n% Flynt stub formal document output\n")?;
            output.pdf = Some(path);
        }
        if String::from_utf8_lossy(&raw).contains("#unknown-function()") {
            output.diagnostics.push(ReportDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "typst_compile_error".to_string(),
                message: "stub Typst engine detected unknown function fixture".to_string(),
                span: None,
            });
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledCliTypstEngine {
    typst_path: PathBuf,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstEngineLocator {
    pub bundled_path: Option<PathBuf>,
    pub allow_system: bool,
}

impl TypstEngineLocator {
    pub fn bundled(path: impl Into<PathBuf>) -> Self {
        Self {
            bundled_path: Some(path.into()),
            allow_system: false,
        }
    }

    pub fn system_for_tests() -> Self {
        Self {
            bundled_path: None,
            allow_system: true,
        }
    }

    pub fn resolve(&self) -> Result<BundledCliTypstEngine> {
        if let Some(path) = &self.bundled_path {
            let version = probe_typst_version(path)?;
            return Ok(BundledCliTypstEngine::new(path, version));
        }
        if self.allow_system {
            let path = PathBuf::from("typst");
            let version = probe_typst_version(&path)?;
            return Ok(BundledCliTypstEngine::new(path, version));
        }
        anyhow::bail!("no bundled Typst binary configured and system Typst fallback is disabled")
    }
}

impl BundledCliTypstEngine {
    pub fn new(typst_path: impl Into<PathBuf>, version: impl Into<String>) -> Self {
        Self {
            typst_path: typst_path.into(),
            version: version.into(),
        }
    }

    fn world_path(&self, request: &FormalDocumentBuildRequest, path: &Path) -> PathBuf {
        if path.is_absolute() || request.world.project_root.as_os_str().is_empty() {
            path.to_path_buf()
        } else {
            request.world.project_root.join(path)
        }
    }

    pub fn preview_args(&self, request: &FormalDocumentBuildRequest) -> Vec<String> {
        let output = request
            .output_dir
            .join("preview")
            .join("page-{0p}-of-{t}.svg");
        self.compile_args(request, &output, true, false)
    }

    pub fn pdf_args(&self, request: &FormalDocumentBuildRequest) -> Vec<String> {
        let output = request.output_dir.join("document.pdf");
        self.compile_args(request, &output, false, false)
    }

    pub fn deps_args(&self, request: &FormalDocumentBuildRequest) -> Vec<String> {
        let output = request.output_dir.join(".deps-probe.pdf");
        self.compile_args(request, &output, false, true)
    }

    fn compile_args(
        &self,
        request: &FormalDocumentBuildRequest,
        output: &Path,
        svg: bool,
        deps: bool,
    ) -> Vec<String> {
        let mut args = vec!["compile".to_string()];
        if svg {
            args.extend(["--format".to_string(), "svg".to_string()]);
        }
        args.extend([
            "--root".to_string(),
            request.world.project_root.to_string_lossy().to_string(),
            "--diagnostic-format".to_string(),
            "short".to_string(),
            "--package-path".to_string(),
            self.world_path(request, &request.world.package_path)
                .to_string_lossy()
                .to_string(),
            "--package-cache-path".to_string(),
            self.world_path(request, &request.world.package_cache_path)
                .to_string_lossy()
                .to_string(),
        ]);
        if request.world.font_mode == TypstFontMode::BundledAndProject {
            args.push("--ignore-system-fonts".to_string());
        }
        for font_path in &request.world.font_paths {
            args.extend([
                "--font-path".to_string(),
                self.world_path(request, font_path)
                    .to_string_lossy()
                    .to_string(),
            ]);
        }
        if let Some(timestamp) = request.world.creation_timestamp {
            args.extend(["--creation-timestamp".to_string(), timestamp.to_string()]);
        }
        for input in &request.inputs {
            args.extend([
                "--input".to_string(),
                format!("{}={}", input.key, input.value),
            ]);
        }
        if deps {
            args.extend([
                "--deps".to_string(),
                self.world_path(request, &request.output_dir.join("deps.json"))
                    .to_string_lossy()
                    .to_string(),
                "--deps-format".to_string(),
                "json".to_string(),
            ]);
        }
        args.push(
            self.world_path(request, &request.source)
                .to_string_lossy()
                .to_string(),
        );
        args.push(
            self.world_path(request, output)
                .to_string_lossy()
                .to_string(),
        );
        args
    }

    fn run_typst(
        &self,
        request: &FormalDocumentBuildRequest,
        args: &[String],
    ) -> Result<Vec<ReportDiagnostic>> {
        let mut command = Command::new(&self.typst_path);
        command.args(args);
        if !request.world.project_root.as_os_str().is_empty() {
            command.current_dir(&request.world.project_root);
        }
        let output = command
            .output()
            .with_context(|| format!("run Typst binary {}", self.typst_path.display()))?;
        if output.status.success() {
            Ok(Vec::new())
        } else {
            let mut combined = output.stderr;
            if !output.stdout.is_empty() {
                if !combined.is_empty() {
                    combined.push(b'\n');
                }
                combined.extend(output.stdout);
            }
            Ok(parse_typst_diagnostics(
                &combined,
                output.status.to_string(),
            ))
        }
    }
}

impl TypstEngine for BundledCliTypstEngine {
    fn engine_info(&self) -> TypstEngineInfo {
        TypstEngineInfo {
            kind: TypstEnginePreference::Bundled,
            version: self.version.clone(),
            path: Some(self.typst_path.clone()),
        }
    }

    fn compile(&self, request: &FormalDocumentBuildRequest) -> Result<TypstEngineOutput> {
        let mut output = TypstEngineOutput::empty();
        if request.preview {
            fs::create_dir_all(request.output_dir.join("preview"))?;
            let diagnostics = self.run_typst(request, &self.preview_args(request))?;
            output.diagnostics.extend(diagnostics);
            if !output
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error)
            {
                output.preview = collect_svg_pages(&request.output_dir.join("preview"))?;
            }
        }
        if request.pdf
            && !output
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error)
        {
            let diagnostics = self.run_typst(request, &self.pdf_args(request))?;
            output.diagnostics.extend(diagnostics);
            let pdf = request.output_dir.join("document.pdf");
            if pdf.exists() {
                output.pdf = Some(pdf);
            }
        }
        if !output
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
        {
            let _ = self.run_typst(request, &self.deps_args(request));
            let deps = request.output_dir.join("deps.json");
            if deps.exists() {
                output
                    .assets
                    .extend(parse_typst_deps(&deps).unwrap_or_else(|_| vec![deps]));
            }
        }
        Ok(output)
    }
}

fn dedup_packages(packages: &mut Vec<TypstPackageUse>) {
    packages.sort_by(|a, b| {
        (&a.namespace, &a.name, &a.version, &a.source).cmp(&(
            &b.namespace,
            &b.name,
            &b.version,
            &b.source,
        ))
    });
    packages.dedup_by(|a, b| {
        a.namespace == b.namespace
            && a.name == b.name
            && a.version == b.version
            && a.source == b.source
    });
}

fn dedup_plugins(plugins: &mut Vec<TypstPluginUse>) {
    plugins.sort_by(|a, b| (&a.sha256, &a.path).cmp(&(&b.sha256, &b.path)));
    plugins.dedup_by(|a, b| a.sha256 == b.sha256 && a.path == b.path);
}

pub fn formal_document_state(
    source: &Path,
    manifest_path: &Path,
) -> Result<FormalDocumentBuildState> {
    if !manifest_path.exists() {
        return Ok(FormalDocumentBuildState::Missing);
    }
    let raw =
        fs::read(source).with_context(|| format!("read formal document {}", source.display()))?;
    let manifest: FormalDocumentBuildManifest = serde_json::from_str(
        &fs::read_to_string(manifest_path)
            .with_context(|| format!("read manifest {}", manifest_path.display()))?,
    )?;
    if manifest.source_sha256 == sha256_hex(&raw) {
        Ok(FormalDocumentBuildState::Clean)
    } else {
        Ok(FormalDocumentBuildState::Dirty)
    }
}

pub fn probe_typst_version(path: &Path) -> Result<String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("probe Typst version with {}", path.display()))?;
    if !output.status.success() {
        anyhow::bail!("typst --version failed with status {}", output.status);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .trim()
        .strip_prefix("typst ")
        .unwrap_or(stdout.trim())
        .to_string())
}

fn parse_typst_diagnostics(stderr: &[u8], status: String) -> Vec<ReportDiagnostic> {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        return vec![ReportDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: "typst_cli_failed".to_string(),
            message: format!("typst exited with status {status}"),
            span: None,
        }];
    }

    // Typst's JSON diagnostic format is line-oriented in current CLI builds.
    // Keep parsing permissive: extract the primary message when the shape is
    // recognized, otherwise preserve stderr verbatim as one diagnostic.
    let mut parsed = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        collect_typst_diagnostics(&value, &mut parsed);
    }
    if parsed.is_empty() {
        for line in text.lines() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            collect_typst_diagnostics(&value, &mut parsed);
        }
    }
    if parsed.is_empty() {
        parsed.push(ReportDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: "typst_cli_failed".to_string(),
            message: text,
            span: None,
        });
    }
    parsed
}

fn collect_typst_diagnostics(value: &serde_json::Value, parsed: &mut Vec<ReportDiagnostic>) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_typst_diagnostics(item, parsed);
            }
        }
        serde_json::Value::Object(_) => {
            let severity = value
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("error");
            let message = value
                .get("message")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("short").and_then(|v| v.as_str()))
                .or_else(|| value.get("body").and_then(|v| v.as_str()));
            if let Some(message) = message {
                parsed.push(ReportDiagnostic {
                    severity: if severity == "warning" {
                        DiagnosticSeverity::Warning
                    } else {
                        DiagnosticSeverity::Error
                    },
                    code: "typst_diagnostic".to_string(),
                    message: message.to_string(),
                    span: None,
                });
            }
        }
        _ => {}
    }
}

fn parse_typst_deps(path: &Path) -> Result<Vec<PathBuf>> {
    let body =
        fs::read_to_string(path).with_context(|| format!("read Typst deps {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&body)?;
    let mut deps = Vec::new();
    collect_dep_paths(&value, &mut deps);
    deps.sort();
    deps.dedup();
    Ok(deps)
}

fn collect_dep_paths(value: &serde_json::Value, deps: &mut Vec<PathBuf>) {
    match value {
        serde_json::Value::String(s)
            if (s.ends_with(".typ")
                || s.ends_with(".svg")
                || s.ends_with(".png")
                || s.ends_with(".jpg")
                || s.ends_with(".jpeg")
                || s.ends_with(".bib")
                || s.ends_with(".yaml")
                || s.ends_with(".yml")
                || s.ends_with(".csl")) =>
        {
            deps.push(PathBuf::from(s));
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_dep_paths(item, deps);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                collect_dep_paths(value, deps);
            }
        }
        _ => {}
    }
}

fn validate_build_request(request: &FormalDocumentBuildRequest) -> Result<()> {
    if request.source.extension().and_then(|ext| ext.to_str()) != Some("typ") {
        anyhow::bail!("formal document source must be a .typ file");
    }
    if request.world.project_root.as_os_str().is_empty() {
        return Ok(());
    }
    let root = normalize_existing_or_parent(&request.world.project_root)?;
    let source = normalize_existing_or_parent(&request.source)?;
    if !source.starts_with(&root) {
        anyhow::bail!(
            "formal document source {} is outside project root {}",
            request.source.display(),
            request.world.project_root.display()
        );
    }
    Ok(())
}

fn normalize_existing_or_parent(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .with_context(|| format!("canonicalize {}", path.display()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    Ok(parent
        .canonicalize()
        .with_context(|| format!("canonicalize {}", parent.display()))?
        .join(path.file_name().unwrap_or_default()))
}

fn previous_successful_preview(output_dir: &Path) -> Vec<PathBuf> {
    let manifest_path = output_dir.join("manifest.json");
    let Ok(body) = fs::read_to_string(&manifest_path) else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<FormalDocumentBuildManifest>(&body) else {
        return Vec::new();
    };
    manifest.outputs.preview
}

fn collect_svg_pages(preview_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut pages = Vec::new();
    if !preview_dir.exists() {
        return Ok(pages);
    }
    for entry in fs::read_dir(preview_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("svg") {
            pages.push(path);
        }
    }
    pages.sort();
    Ok(pages)
}

fn stub_svg(source: &Path) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><text x="24" y="48">Flynt formal document preview: {}</text></svg>"#,
        source.display()
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn duration_ms(duration: Duration) -> u128 {
    duration.as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_world_policy_is_reproducible_first() {
        let policy = TypstWorldPolicy::default();
        assert_eq!(policy.engine_preference, TypstEnginePreference::Bundled);
        assert_eq!(policy.package_mode, TypstPackageMode::AskBeforeDownload);
        assert_eq!(policy.font_mode, TypstFontMode::BundledAndProject);
        assert_eq!(policy.plugin_mode, TypstPluginMode::AskBeforeFirstHash);
    }

    #[test]
    fn build_service_writes_formal_document_bundle_contract() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/brief.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "= Brief\n\nHello.").unwrap();
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/brief"));
        request.world.project_root = dir.path().to_path_buf();
        let service =
            FormalDocumentBuildService::new(StubTypstEngine::bundled_for_tests("0.test.0"));

        let result = service.build(&request).unwrap();

        assert_eq!(result.state, FormalDocumentBuildState::Succeeded);
        assert_eq!(result.manifest.kind, "formal_document_build");
        assert_eq!(result.manifest.engine.version, "0.test.0");
        assert!(result.manifest.outputs.manifest.exists());
        assert_eq!(result.manifest.outputs.preview.len(), 1);
        assert!(result.manifest.outputs.preview[0].exists());
    }

    #[test]
    fn failed_build_keeps_previous_successful_preview_slot() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/bad.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "= Good").unwrap();
        let mut request = FormalDocumentBuildRequest::new(&source, dir.path().join("reports/bad"));
        request.world.project_root = dir.path().to_path_buf();
        let service =
            FormalDocumentBuildService::new(StubTypstEngine::bundled_for_tests("0.test.0"));
        let first = service.build(&request).unwrap();
        assert_eq!(first.state, FormalDocumentBuildState::Succeeded);

        fs::write(&source, "#unknown-function()").unwrap();
        let failed = service.build(&request).unwrap();

        assert_eq!(failed.state, FormalDocumentBuildState::Failed);
        assert!(!failed.manifest.diagnostics.is_empty());
        assert!(failed.manifest.outputs.preview.is_empty());
        assert_eq!(
            failed.manifest.outputs.last_successful_preview,
            first.manifest.outputs.preview
        );
    }

    #[test]
    fn bundled_cli_args_encode_world_policy() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/brief.typ");
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/brief"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.creation_timestamp = Some(1_700_000_000);
        request.inputs.push(TypstInput {
            key: "flynt_document".into(),
            value: "documents/brief.typ".into(),
        });
        let engine = BundledCliTypstEngine::new("/opt/flynt/typst", "0.test.0");

        let args = engine.preview_args(&request);

        assert!(
            args.windows(2)
                .any(|w| w == ["--root", dir.path().to_string_lossy().as_ref()])
        );
        assert!(args.windows(2).any(|w| {
            w == [
                "--package-path",
                dir.path()
                    .join(".flynt/typst/packages")
                    .to_string_lossy()
                    .as_ref(),
            ]
        }));
        assert!(args.contains(&"--ignore-system-fonts".to_string()));
        assert!(
            args.windows(2)
                .any(|w| w == ["--diagnostic-format", "short"])
        );
        assert!(
            args.windows(2)
                .any(|w| w == ["--creation-timestamp", "1700000000"])
        );
        assert!(
            args.windows(2)
                .any(|w| w == ["--input", "flynt_document=documents/brief.typ"])
        );
        assert!(
            args.last()
                .is_some_and(|arg| arg.ends_with("page-{0p}-of-{t}.svg"))
        );
    }

    #[test]
    fn system_font_mode_does_not_emit_ignore_system_fonts() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/brief.typ");
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/brief"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.font_mode = TypstFontMode::BundledProjectAndSystem;
        let engine = BundledCliTypstEngine::new("/opt/flynt/typst", "0.test.0");

        let args = engine.preview_args(&request);

        assert!(!args.contains(&"--ignore-system-fonts".to_string()));
    }

    #[test]
    fn formal_document_state_reports_missing_clean_and_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/state.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "= State").unwrap();
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/state"));
        request.world.project_root = dir.path().to_path_buf();
        let manifest_path = request.output_dir.join("manifest.json");
        assert_eq!(
            formal_document_state(&source, &manifest_path).unwrap(),
            FormalDocumentBuildState::Missing
        );
        let service =
            FormalDocumentBuildService::new(StubTypstEngine::bundled_for_tests("0.test.0"));
        service.build(&request).unwrap();
        assert_eq!(
            formal_document_state(&source, &manifest_path).unwrap(),
            FormalDocumentBuildState::Clean
        );
        fs::write(&source, "= State\n\nChanged.").unwrap();
        assert_eq!(
            formal_document_state(&source, &manifest_path).unwrap(),
            FormalDocumentBuildState::Dirty
        );
    }

    #[test]
    fn parses_line_oriented_typst_json_diagnostics() {
        let stderr = br#"{"severity":"error","message":"unknown variable: foo"}
"#;
        let diagnostics = parse_typst_diagnostics(stderr, "exit status: 1".to_string());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "typst_diagnostic");
        assert!(diagnostics[0].message.contains("unknown variable"));
    }

    #[test]
    fn parses_typst_deps_json_permissively() {
        let dir = tempfile::tempdir().unwrap();
        let deps = dir.path().join("deps.json");
        fs::write(
            &deps,
            r#"{"files":["documents/main.typ","assets/figure.svg","notes/readme.md"]}"#,
        )
        .unwrap();
        let parsed = parse_typst_deps(&deps).unwrap();
        assert!(parsed.contains(&PathBuf::from("documents/main.typ")));
        assert!(parsed.contains(&PathBuf::from("assets/figure.svg")));
        assert!(!parsed.contains(&PathBuf::from("notes/readme.md")));
    }

    #[test]
    fn system_typst_locator_is_non_fatal_when_binary_missing() {
        let locator = TypstEngineLocator::system_for_tests();
        let result = locator.resolve();
        if let Err(err) = result {
            eprintln!("skipping system Typst assertion: {err}");
        }
    }

    #[test]
    fn real_typst_cli_compiles_minimal_fixture_when_available() {
        let locator = TypstEngineLocator::system_for_tests();
        let Ok(engine) = locator.resolve() else {
            eprintln!("skipping real Typst compile fixture: system typst not available");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/minimal.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "= Minimal\n\nHello from Flynt.\n").unwrap();
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/minimal"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.package_path = PathBuf::from(".flynt/typst/packages");
        request.world.package_cache_path = PathBuf::from(".flynt/cache/typst/packages");
        request.world.font_paths.clear();
        request.pdf = false;
        let service = FormalDocumentBuildService::new(engine);

        let result = service.build(&request).unwrap();

        assert_eq!(
            result.state,
            FormalDocumentBuildState::Succeeded,
            "diagnostics: {:?}",
            result.manifest.diagnostics
        );
        assert!(!result.manifest.outputs.preview.is_empty());
        assert!(
            result
                .manifest
                .outputs
                .preview
                .iter()
                .all(|path| path.exists())
        );
        assert!(!result.manifest.engine.version.is_empty());
    }

    fn fixture_typst_engine() -> Option<BundledCliTypstEngine> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir.parent()?.parent()?.to_path_buf();
        let project_tool = root.join(".flynt/typst-toolchain/bin/typst");
        if project_tool.exists() {
            return TypstEngineLocator::bundled(project_tool).resolve().ok();
        }
        TypstEngineLocator::system_for_tests().resolve().ok()
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn real_typst_cli_compiles_core_fixture_suite_when_available() {
        let Some(engine) = fixture_typst_engine() else {
            eprintln!("skipping real Typst fixture suite: typst binary not available");
            return;
        };
        let root = repo_root();
        let fixture_dir = root.join("fixtures/formal-documents");
        let out_root = tempfile::tempdir().unwrap();
        let passing = [
            "minimal",
            "math",
            "unicode",
            "multi-page",
            "figures",
            "bibliography",
            "project-font",
        ];

        for name in passing {
            let source = fixture_dir.join(format!("{name}.typ"));
            let mut request = FormalDocumentBuildRequest::new(&source, out_root.path().join(name));
            request.world.project_root = fixture_dir.clone();
            request.world.package_path = fixture_dir.join(".flynt/typst/packages");
            request.world.package_cache_path = fixture_dir.join(".flynt/cache/typst/packages");
            request.world.font_paths = vec![fixture_dir.join("fonts")];
            let service = FormalDocumentBuildService::new(engine.clone());
            let result = service.build(&request).unwrap();
            assert_eq!(
                result.state,
                FormalDocumentBuildState::Succeeded,
                "fixture {name} diagnostics: {:?}",
                result.manifest.diagnostics
            );
            assert!(
                !result.manifest.outputs.preview.is_empty(),
                "fixture {name} wrote no preview pages"
            );
            for page in &result.manifest.outputs.preview {
                assert!(
                    page.exists(),
                    "fixture {name} preview page missing: {}",
                    page.display()
                );
            }
        }
    }

    #[test]
    fn real_typst_cli_preserves_preview_on_compile_error_when_available() {
        let Some(engine) = fixture_typst_engine() else {
            eprintln!("skipping real Typst failure fixture: typst binary not available");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("doc.typ");
        fs::write(&source, "= Good\n\nThis compiles.").unwrap();
        let mut request = FormalDocumentBuildRequest::new(&source, dir.path().join("reports/doc"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.package_path = dir.path().join(".flynt/typst/packages");
        request.world.package_cache_path = dir.path().join(".flynt/cache/typst/packages");
        request.world.font_paths = vec![dir.path().join("fonts")];
        let service = FormalDocumentBuildService::new(engine);
        let first = service.build(&request).unwrap();
        assert_eq!(
            first.state,
            FormalDocumentBuildState::Succeeded,
            "initial diagnostics: {:?}",
            first.manifest.diagnostics
        );
        assert!(!first.manifest.outputs.preview.is_empty());

        fs::write(&source, "= Broken\n\n#unknown-function()").unwrap();
        let failed = service.build(&request).unwrap();
        assert_eq!(failed.state, FormalDocumentBuildState::Failed);
        assert!(failed.manifest.outputs.preview.is_empty());
        assert_eq!(
            failed.manifest.outputs.last_successful_preview,
            first.manifest.outputs.preview
        );
        assert!(!failed.manifest.diagnostics.is_empty());
    }

    #[test]
    fn preflight_reports_missing_package_in_offline_mode() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("doc.typ");
        fs::write(&source, "#import \"@local/missing:0.1.0\": *").unwrap();
        let world = TypstWorldPolicy {
            project_root: dir.path().to_path_buf(),
            package_path: dir.path().join(".flynt/typst/packages"),
            package_mode: TypstPackageMode::OfflineOnly,
            ..TypstWorldPolicy::default()
        };
        let preflight = preflight_typst_policy(&source, &world, None, None).unwrap();
        assert!(
            preflight
                .diagnostics
                .iter()
                .any(|d| d.code == "typst_package_missing")
        );
    }

    #[test]
    fn preflight_hashes_project_local_package_and_requires_plugin_approval() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("doc.typ");
        fs::write(&source, "#import \"@local/flynt-fixture:0.1.0\": *").unwrap();
        let package = dir
            .path()
            .join(".flynt/typst/packages/local/flynt-fixture/0.1.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.typ"), "#let answer = 42").unwrap();
        fs::write(
            package.join("plugin.wasm"),
            b"not-real-wasm-but-policy-bytes",
        )
        .unwrap();
        let world = TypstWorldPolicy {
            project_root: dir.path().to_path_buf(),
            package_path: dir.path().join(".flynt/typst/packages"),
            ..TypstWorldPolicy::default()
        };
        let preflight = preflight_typst_policy(&source, &world, None, None).unwrap();
        assert_eq!(preflight.packages.len(), 1);
        assert!(preflight.packages[0].sha256.is_some());
        assert_eq!(preflight.plugins.len(), 1);
        assert!(
            preflight
                .diagnostics
                .iter()
                .any(|d| d.code == "typst_plugin_unapproved")
        );
        let approvals = TypstPluginApprovals {
            version: 1,
            approvals: vec![TypstPluginApproval {
                sha256: preflight.plugins[0].sha256.clone(),
                source: "test".into(),
                path: preflight.plugins[0].path.clone(),
                reason: None,
            }],
        };
        let approved = preflight_typst_policy(&source, &world, None, Some(&approvals)).unwrap();
        assert!(
            approved
                .diagnostics
                .iter()
                .all(|d| d.code != "typst_plugin_unapproved")
        );
        fs::write(package.join("plugin.wasm"), b"changed").unwrap();
        let changed = preflight_typst_policy(&source, &world, None, Some(&approvals)).unwrap();
        assert!(
            changed
                .diagnostics
                .iter()
                .any(|d| d.code == "typst_plugin_unapproved")
        );
    }

    #[test]
    fn build_service_stops_before_compile_on_policy_error() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/policy.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "#import \"@local/missing:0.1.0\": *").unwrap();
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/policy"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.package_path = dir.path().join(".flynt/typst/packages");
        request.world.package_mode = TypstPackageMode::OfflineOnly;
        let service =
            FormalDocumentBuildService::new(StubTypstEngine::bundled_for_tests("0.test.0"));

        let result = service.build(&request).unwrap();

        assert_eq!(result.state, FormalDocumentBuildState::Failed);
        assert!(
            result
                .manifest
                .diagnostics
                .iter()
                .any(|d| d.code == "typst_package_missing")
        );
        assert!(result.manifest.outputs.preview.is_empty());
    }

    #[test]
    fn build_manifest_includes_policy_preflight_packages_and_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("documents/policy-ok.typ");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "#import \"@local/flynt-fixture:0.1.0\": *").unwrap();
        let package = dir
            .path()
            .join(".flynt/typst/packages/local/flynt-fixture/0.1.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.typ"), "#let answer = 42").unwrap();
        fs::write(package.join("plugin.wasm"), b"plugin bytes").unwrap();
        let plugin_hash = hash_file(&package.join("plugin.wasm")).unwrap();
        let mut request =
            FormalDocumentBuildRequest::new(&source, dir.path().join("reports/policy-ok"));
        request.world.project_root = dir.path().to_path_buf();
        request.world.package_path = dir.path().join(".flynt/typst/packages");
        request.plugin_approvals = Some(TypstPluginApprovals {
            version: 1,
            approvals: vec![TypstPluginApproval {
                sha256: plugin_hash.clone(),
                source: "test".into(),
                path: package.join("plugin.wasm"),
                reason: Some("test".into()),
            }],
        });
        let service =
            FormalDocumentBuildService::new(StubTypstEngine::bundled_for_tests("0.test.0"));

        let result = service.build(&request).unwrap();

        assert_eq!(result.state, FormalDocumentBuildState::Succeeded);
        assert_eq!(result.manifest.packages.len(), 1);
        assert_eq!(result.manifest.plugins.len(), 1);
        assert_eq!(result.manifest.plugins[0].sha256, plugin_hash);
        assert!(result.manifest.plugins[0].approved);
    }

    #[test]
    fn formal_document_settings_map_to_world_policy() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FormalDocumentSettings {
            network_mode: TypstNetworkMode::OfflineOnly,
            font_mode: TypstFontMode::BundledProjectAndSystem,
            creation_timestamp: Some(123),
            ..FormalDocumentSettings::default()
        };

        let world = settings.world_policy(dir.path());

        assert_eq!(world.package_mode, TypstPackageMode::OfflineOnly);
        assert_eq!(world.font_mode, TypstFontMode::BundledProjectAndSystem);
        assert_eq!(world.creation_timestamp, Some(123));
        assert!(world.package_path.starts_with(dir.path()));
    }

    #[test]
    fn formal_document_doctor_reports_engine_disabled_and_approval_count() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FormalDocumentSettings {
            engine_mode: FormalDocumentEngineMode::Disabled,
            ..FormalDocumentSettings::default()
        };
        let approvals = TypstPluginApprovals {
            version: 1,
            approvals: vec![TypstPluginApproval {
                sha256: "abc".into(),
                source: "test".into(),
                path: PathBuf::from("plugin.wasm"),
                reason: None,
            }],
        };

        let report = formal_document_doctor(settings, dir.path(), None, Some(&approvals));

        assert!(!report.engine.ok);
        assert_eq!(report.plugin_approval_count, 1);
        assert_eq!(report.font_paths.len(), 3);
    }
}
