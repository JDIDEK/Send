use std::path::{Component, Path, PathBuf};

use iroh::ticket::BlobTicket;

pub fn sanitize_relative_path(name: &str) -> PathBuf {
    let mut sanitized = PathBuf::new();

    for component in Path::new(name).components() {
        if let Component::Normal(part) = component {
            sanitized.push(part);
        }
    }

    if sanitized.as_os_str().is_empty() {
        sanitized.push("fichier");
    }

    sanitized
}

pub fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fichier".to_string());
    let extension = path.extension().map(|value| value.to_string_lossy().into_owned());

    for index in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("un chemin libre finit toujours par être trouvé")
}

pub fn short_hash(ticket: &BlobTicket) -> String {
    ticket.hash().to_string().chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_relative_path_drops_parent_segments() {
        assert_eq!(sanitize_relative_path("../unsafe/file.txt"), PathBuf::from("unsafe/file.txt"));
    }

    #[test]
    fn sanitize_relative_path_falls_back_to_default_name() {
        assert_eq!(sanitize_relative_path("../.."), PathBuf::from("fichier"));
    }
}
