use std::{fs, io};

use elfcode_core::layout;

use crate::project::Project;

const FILE: &str = ".gitignore";

/// Makes sure the project ignores downloaded puzzle files, which must not
/// be published. Adds only the patterns that are missing and keeps the rest
/// of the file as it is.
pub fn ensure_downloads_ignored(project: &Project) -> io::Result<()> {
    let path = project.path(FILE);
    let mut contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };

    let missing: Vec<_> = layout::gitignore_patterns()
        .into_iter()
        .filter(|pattern| !contents.lines().any(|line| line.trim() == pattern))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }

    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    for pattern in missing {
        contents.push_str(&pattern);
        contents.push('\n');
    }

    fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    use super::*;

    fn project() -> (TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::at(Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap());
        (dir, project)
    }

    fn gitignore(project: &Project) -> String {
        fs::read_to_string(project.path(FILE)).unwrap()
    }

    #[test]
    fn creates_the_file_when_there_is_none() {
        let (_dir, project) = project();

        ensure_downloads_ignored(&project).unwrap();

        assert_eq!(
            gitignore(&project),
            "/data/**/input.txt\n/data/**/puzzle.md\n"
        );
    }

    #[test]
    fn adds_only_what_is_missing_and_keeps_the_rest() {
        let (_dir, project) = project();
        fs::write(project.path(FILE), "/target\n/data/**/input.txt").unwrap();

        ensure_downloads_ignored(&project).unwrap();

        assert_eq!(
            gitignore(&project),
            "/target\n/data/**/input.txt\n/data/**/puzzle.md\n"
        );
    }

    #[test]
    fn leaves_a_complete_file_untouched() {
        let (_dir, project) = project();
        ensure_downloads_ignored(&project).unwrap();
        let before = fs::metadata(project.path(FILE))
            .unwrap()
            .modified()
            .unwrap();

        ensure_downloads_ignored(&project).unwrap();

        let after = fs::metadata(project.path(FILE))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(before, after);
    }
}
