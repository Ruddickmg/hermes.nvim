use std::{collections::HashMap, path::PathBuf};

pub fn get_project_root(current_directory: PathBuf, root_markers: Vec<String>) -> PathBuf {
    let markers: HashMap<String, bool> =
        root_markers.iter().map(|m| (m.to_string(), true)).collect();
    let buf = current_directory.ancestors().find(|dir| {
        dir.read_dir()
            .map(|mut files| {
                files.any(|file| {
                    file.map(|details| {
                        details
                            .file_name()
                            .into_string()
                            .map(|file_name| markers.contains_key(&file_name))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });
    buf.map(PathBuf::from).unwrap_or(current_directory)
}

#[cfg(test)]
mod get_project_root_tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    // get_project_root Tests

    #[test]
    fn test_get_project_root_current_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let marker = root.join(".git");
        File::create(&marker).unwrap();

        let found_root = get_project_root(root.clone(), vec![".git".to_string()]);
        assert_eq!(found_root, root);
    }

    #[test]
    fn test_get_project_root_parent_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let marker = root.join(".git");
        File::create(&marker).unwrap();

        let subdir = root.join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let found_root = get_project_root(subdir, vec![".git".to_string()]);
        assert_eq!(found_root, root);
    }

    #[test]
    fn test_get_project_root_grandparent_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let marker = root.join(".git");
        File::create(&marker).unwrap();

        let subdir = root.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let subsubdir = subdir.join("nested");
        std::fs::create_dir(&subsubdir).unwrap();

        let found_root = get_project_root(subsubdir, vec![".git".to_string()]);
        assert_eq!(found_root, root);
    }

    #[test]
    fn test_get_project_root_no_marker_returns_start_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let subdir = root.join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let found_root = get_project_root(subdir.clone(), vec![".git".to_string()]);
        assert_eq!(found_root, subdir);
    }

    #[test]
    fn test_get_project_root_multiple_markers() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        // Use a different marker
        let marker = root.join("Cargo.toml");
        File::create(&marker).unwrap();

        let subdir = root.join("src");
        std::fs::create_dir(&subdir).unwrap();

        let found_root =
            get_project_root(subdir, vec![".git".to_string(), "Cargo.toml".to_string()]);
        assert_eq!(found_root, root);
    }

    #[test]
    fn test_get_project_root_handles_nonexistent_start_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let marker = root.join(".git");
        File::create(&marker).unwrap();

        let non_existent = root.join("does_not_exist");

        let found_root = get_project_root(non_existent, vec![".git".to_string()]);
        assert_eq!(found_root, root);
    }
}
