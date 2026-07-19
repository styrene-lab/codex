use flynt_store::project::Project;
use std::path::PathBuf;

#[test]
fn candidate_snapshot_opens_and_reindexes() {
    let Some(root) = std::env::var_os("FLYNT_CANDIDATE_SMOKE_PROJECT").map(PathBuf::from) else {
        eprintln!("skipping: FLYNT_CANDIDATE_SMOKE_PROJECT is not set");
        return;
    };

    assert!(root.join(".flynt-candidate-source.json").is_file());
    assert!(root.join(".flynt-candidate-manifest.json").is_file());
    assert!(!root.join(".git").exists());

    let project = Project::open(&root).expect("Candidate snapshot should open as a Flynt project");
    let (_indexed, errors) = project
        .reindex()
        .expect("Candidate snapshot should complete a full reindex");
    assert!(
        errors.is_empty(),
        "Candidate snapshot reindex reported errors: {errors:?}"
    );
}
