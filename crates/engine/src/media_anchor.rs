//! Ours (not upstream): media follow a moved project folder.
//!
//! A project file records the folder it was saved in (`saved_dir`, beside `view` in the file's
//! envelope; [`filmcraft_format::encode_at`]). Media paths stay absolute, as before. When a project
//! is opened from another folder, each media file (and proxy) is looked for at the same place
//! relative to the project as when it was saved: `relative(path, saved_dir)` joined to the folder
//! it is opened from. A file found there with the recorded size (when one is recorded) replaces
//! the absolute path; otherwise the absolute path stays, and whatever is still missing goes to
//! Link Media as before. So a show folder moved or copied with its media (another drive, another
//! machine, Collect Files) opens without relinking.
//!
//! Relative first, and only when the project has moved: the copy beside the moved project is the
//! one meant, even when the original still exists. The size check keeps a different file at that
//! place from being taken. Paths are compared lexically, splitting on `/` and `\`, so a project
//! saved on Windows opens on Linux and the other way round. Media on another drive than the project
//! (no shared folder) has no relative place and keeps its absolute path.

use filmcraft_project::{ItemKind, MediaRef, Project};

use crate::{Services, Session};

/// The folder containing `path` (a project file), when `path` is absolute.
pub fn folder_of(path: &str) -> Option<String> {
    if !is_absolute(path) {
        return None;
    }
    let cut = path.rfind(['/', '\\'])?;
    let dir = &path[..cut];
    // the root itself: keep its separator ("/", "C:\")
    Some(if dir.is_empty() || dir.ends_with(':') { path[..=cut].to_string() } else { dir.to_string() })
}

/// Absolute on Unix (`/…`) or Windows (`C:\…`, `C:/…`, `\\server\…`).
fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/') || p.starts_with("\\\\") || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/'))
}

/// The root (`/`, `C:`, `\\server`) and the folder names of an absolute path.
fn parts(p: &str) -> (String, Vec<&str>) {
    let root = if let Some(rest) = p.strip_prefix("\\\\") {
        format!("\\\\{}", rest.split(['/', '\\']).next().unwrap_or(""))
    } else if p.starts_with('/') {
        "/".to_string()
    } else {
        p[..2].to_string()
    };
    let body = &p[root.len().min(p.len())..];
    let names = body.split(['/', '\\']).filter(|c| !c.is_empty() && *c != ".").collect();
    (root, names)
}

/// Where `file` is relative to the folder `dir` (both absolute): `..` steps up, then names.
/// `None` on different roots (drives), or when they share no folder below the root (`/home/…`
/// against `/run/media/…`): such media did not live with the project.
pub fn relative(file: &str, dir: &str) -> Option<Vec<String>> {
    if !is_absolute(file) || !is_absolute(dir) {
        return None;
    }
    let ((fr, f), (dr, d)) = (parts(file), parts(dir));
    if fr != dr || f.is_empty() {
        return None;
    }
    let common = f.iter().zip(&d).take_while(|(a, b)| a == b).count();
    if common == 0 || common == f.len() {
        return None;
    }
    let mut out: Vec<String> = std::iter::repeat_n("..".to_string(), d.len() - common).collect();
    out.extend(f[common..].iter().map(|s| s.to_string()));
    Some(out)
}

/// `rel` ([`relative`]) joined to the folder `dir`, written with `dir`'s separator.
pub fn resolve(rel: &[String], dir: &str) -> String {
    let sep = if dir.contains('\\') && !dir.contains('/') { '\\' } else { '/' };
    let (root, mut names) = parts(dir);
    let mut names: Vec<String> = names.drain(..).map(str::to_string).collect();
    for c in rel {
        if c == ".." {
            names.pop();
        } else {
            names.push(c.clone());
        }
    }
    let root = if root == "/" { String::new() } else { root };
    format!("{root}{sep}{}", names.join(&sep.to_string()))
}

/// Whether two folders are the same place (separators, a trailing separator and, for drive
/// letters, case aside).
fn same_folder(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        let t = s.replace('\\', "/");
        let t = t.trim_end_matches('/').to_string();
        if t.as_bytes().get(1) == Some(&b':') { t.to_lowercase() } else { t }
    };
    norm(a) == norm(b)
}

/// On open: point each media file and proxy at its place beside the project, when the project
/// was saved in `saved_dir` and is opened from `project_path` somewhere else and the file is
/// there (with its recorded size, when one is recorded). How many paths changed.
pub fn follow(project: &mut Project, saved_dir: Option<&str>, project_path: &str, services: &dyn Services) -> usize {
    let (Some(saved), Some(now)) = (saved_dir, folder_of(project_path)) else { return 0 };
    if same_folder(saved, &now) {
        return 0;
    }
    let mut changed = 0;
    let candidate = |path: &str, size: Option<u64>| -> Option<String> {
        let cand = resolve(&relative(path, saved)?, &now);
        let got = services.file_size(&cand).ok()?;
        (size.is_none_or(|s| s == got) && cand != path).then_some(cand)
    };
    for it in project.items.values_mut() {
        let ItemKind::Media(m) = &mut it.kind else { continue };
        let size = m.identity.map(|i| i.size);
        if let MediaRef::File { path } = &mut m.media
            && let Some(cand) = candidate(path, size)
        {
            log::info!("decision: media: \"{path}\" followed the project folder to \"{cand}\"");
            *path = cand;
            changed += 1;
        }
        if let Some(MediaRef::File { path }) = &mut m.proxy
            && let Some(cand) = candidate(path, None)
        {
            log::info!("decision: media: proxy \"{path}\" followed the project folder to \"{cand}\"");
            *path = cand;
            changed += 1;
        }
    }
    changed
}

/// Tell the user how many media paths followed the project folder (none: nothing).
pub fn report(s: &mut Session, followed: usize) {
    if followed > 0 {
        s.toast(format!("{followed} media file{} followed the project folder", if followed == 1 { "" } else { "s" }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(f: &str, d: &str) -> Option<String> {
        relative(f, d).map(|v| v.join("/"))
    }

    #[test]
    fn relative_places_on_unix_and_windows() {
        assert_eq!(rel("/shows/ep1/media/a.mov", "/shows/ep1").as_deref(), Some("media/a.mov"));
        assert_eq!(rel("/shows/ep1/media/a.mov", "/shows/ep1/projects").as_deref(), Some("../media/a.mov"));
        assert_eq!(rel(r"J:\Shows\Ep1\Media\a.mov", r"J:\Shows\Ep1\Projects").as_deref(), Some("../Media/a.mov"));
        assert_eq!(rel("J:/Shows/Ep1/a.mov", r"J:\Shows\Ep1").as_deref(), Some("a.mov"), "either separator");
        // no place relative to the project: another drive, or only the root in common
        assert_eq!(rel(r"D:\Media\a.mov", r"C:\Shows\Ep1"), None);
        assert_eq!(rel("/run/media/x/a.mov", "/home/sam/ep1"), None);
        assert_eq!(rel("media/a.mov", "/shows"), None, "not absolute");
    }

    #[test]
    fn resolve_joins_with_the_folders_separator() {
        let r = |s: &str| s.split('/').map(str::to_string).collect::<Vec<_>>();
        assert_eq!(resolve(&r("../Media/a.mov"), r"K:\Moved\Ep1\Projects"), r"K:\Moved\Ep1\Media\a.mov");
        assert_eq!(resolve(&r("media/a.mov"), "/mnt/usb/ep1"), "/mnt/usb/ep1/media/a.mov");
        // saved on Windows, opened on Linux
        assert_eq!(resolve(&relative(r"J:\Shows\Ep1\Media\a.mov", r"J:\Shows\Ep1").unwrap(), "/home/sam/Ep1"), "/home/sam/Ep1/Media/a.mov");
    }

    #[test]
    fn folder_of_a_project_file() {
        assert_eq!(folder_of("/shows/ep1/ep1.fcproj").as_deref(), Some("/shows/ep1"));
        assert_eq!(folder_of(r"J:\Shows\Ep1\ep1.fcproj").as_deref(), Some(r"J:\Shows\Ep1"));
        assert_eq!(folder_of("/ep1.fcproj").as_deref(), Some("/"));
        assert_eq!(folder_of(r"C:\ep1.fcproj").as_deref(), Some(r"C:\"));
        assert_eq!(folder_of("ep1.fcproj"), None, "a web blob name");
        assert!(same_folder(r"J:\Shows\Ep1\", "j:/Shows/Ep1"));
        assert!(!same_folder("/Shows/Ep1", "/shows/ep1"), "case matters on Unix");
    }

    use filmcraft_media::DemoScene;
    use serde_json::json;

    use crate::media_test_util::{make_movie, session_with, tmp_dir};

    /// `<root>/Project/p.fcproj` using `<root>/Media/{a,b}.mov`, saved; its path.
    fn saved_show(root: &std::path::Path) -> String {
        let media = root.join("Media");
        std::fs::create_dir_all(&media).unwrap();
        let (a, b) = (media.join("a.mov"), media.join("b.mov"));
        make_movie(&a, DemoScene::Aurora, 160, 90, 12);
        make_movie(&b, DemoScene::CityNight, 160, 90, 12);
        let (mut s, _, _) = session_with(&[&a, &b]);
        std::fs::create_dir_all(root.join("Project")).unwrap();
        let path = root.join("Project").join("p.fcproj").to_string_lossy().into_owned();
        s.execute("file.save", json!({"path": path})).unwrap();
        path
    }

    fn media_paths(s: &Session) -> Vec<String> {
        let mut v: Vec<String> = s
            .project
            .items
            .values()
            .filter_map(|it| match &it.kind {
                ItemKind::Media(m) => match &m.media {
                    MediaRef::File { path } => Some(path.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_project_folder_moved_with_its_media_opens_with_nothing_missing() {
        let root = tmp_dir("anchor-moved");
        let old = root.join("Show");
        let path = saved_show(&old);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"saved_dir\""), "the file records its folder");
        let new = root.join("Elsewhere").join("Show");
        std::fs::create_dir_all(new.parent().unwrap()).unwrap();
        std::fs::rename(&old, &new).unwrap();
        let moved = new.join("Project").join("p.fcproj").to_string_lossy().into_owned();
        let mut s = Session::default();
        let r = s.execute("file.open", json!({"path": moved})).unwrap();
        assert_eq!(r["missingMedia"], json!(0), "{r}");
        let want: Vec<String> = ["a.mov", "b.mov"].iter().map(|f| new.join("Media").join(f).to_string_lossy().into_owned()).collect();
        assert_eq!(media_paths(&s), want);
        assert!(!s.is_dirty(), "following the folder is not an edit");
    }

    #[test]
    fn a_different_file_at_the_moved_place_is_not_taken() {
        let root = tmp_dir("anchor-other-file");
        let old = root.join("Show");
        let path = saved_show(&old);
        let new = root.join("Moved");
        std::fs::rename(&old, &new).unwrap();
        // b.mov beside the moved project is another file (another size)
        std::fs::write(new.join("Media").join("b.mov"), b"not the same movie").unwrap();
        let moved = new.join("Project").join("p.fcproj").to_string_lossy().into_owned();
        let mut s = Session::default();
        let r = s.execute("file.open", json!({"path": moved})).unwrap();
        assert_eq!(r["missingMedia"], json!(1), "b.mov stays at its old (now missing) path: {r}");
        let p = media_paths(&s);
        assert!(p.contains(&new.join("Media").join("a.mov").to_string_lossy().into_owned()), "{p:?}");
        assert!(p.contains(&old.join("Media").join("b.mov").to_string_lossy().into_owned()), "{p:?}");
        let _ = path;
    }

    #[test]
    fn an_older_file_without_the_folder_opens_as_before() {
        let root = tmp_dir("anchor-older");
        let old = root.join("Show");
        let path = saved_show(&old);
        // as an older build writes it: no saved_dir
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut doc = doc;
        doc.as_object_mut().unwrap().remove("saved_dir");
        std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        let new = root.join("Moved");
        std::fs::rename(&old, &new).unwrap();
        let mut s = Session::default();
        let r = s.execute("file.open", json!({"path": new.join("Project").join("p.fcproj").to_string_lossy()})).unwrap();
        assert_eq!(r["missingMedia"], json!(2), "{r}");
    }
}
