//! Integration tests for project utilities
use hermes::utilities::get_project_root;
use std::path::PathBuf;

#[nvim_oxi::test]
fn test_get_project_root_finds_git_repo() -> nvim_oxi::Result<()> {
    let current_dir = std::env::current_dir().unwrap();
    let root_markers = vec![".git".to_string()];

    let project_root = get_project_root(current_dir.clone(), root_markers);

    // Should find the git repository root
    assert!(
        project_root.join(".git").exists() || project_root.join("Cargo.toml").exists(),
        "Project root should contain .git or Cargo.toml"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_get_project_root_is_ancestor_of_current() -> nvim_oxi::Result<()> {
    let current_dir = std::env::current_dir().unwrap();
    let root_markers = vec![".git".to_string()];

    let project_root = get_project_root(current_dir.clone(), root_markers);

    // Should be an ancestor of current directory
    assert!(
        current_dir.starts_with(&project_root),
        "Project root should be ancestor of current directory"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_get_project_root_finds_cargo_toml() -> nvim_oxi::Result<()> {
    let current_dir = std::env::current_dir().unwrap();
    let root_markers = vec!["Cargo.toml".to_string()];

    let project_root = get_project_root(current_dir.clone(), root_markers);

    // Should find directory with Cargo.toml
    assert!(
        project_root.join("Cargo.toml").exists(),
        "Project root should contain Cargo.toml"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_get_project_root_fallback_to_start() -> nvim_oxi::Result<()> {
    // Use markers that don't exist
    let current_dir = PathBuf::from("/tmp");
    let root_markers = vec!["nonexistent_marker.xyz".to_string()];

    let project_root = get_project_root(current_dir.clone(), root_markers);

    // Should return the start directory if no markers found
    assert_eq!(
        project_root, current_dir,
        "Should return start directory when no markers found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_get_project_root_multiple_markers() -> nvim_oxi::Result<()> {
    let current_dir = std::env::current_dir().unwrap();
    let root_markers = vec![
        ".git".to_string(),
        "Cargo.toml".to_string(),
        "package.json".to_string(),
    ];

    let project_root = get_project_root(current_dir.clone(), root_markers.clone());

    // Should find at least one marker
    let has_marker = root_markers
        .iter()
        .any(|marker| project_root.join(marker).exists());
    assert!(
        has_marker,
        "Project root should contain at least one marker"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_get_project_root_traverses_parents() -> nvim_oxi::Result<()> {
    // Start from a subdirectory
    let base_dir = std::env::current_dir().unwrap();
    let subdir = base_dir.join("src").join("nvim");

    if subdir.exists() {
        let root_markers = vec![".git".to_string()];
        let project_root = get_project_root(subdir.clone(), root_markers);

        // Should traverse up and find root
        assert!(
            project_root.join(".git").exists(),
            "Should find .git in parent directories"
        );
    }

    Ok(())
}
