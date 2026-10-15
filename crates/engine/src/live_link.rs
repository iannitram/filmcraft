//! Ours (not upstream): live links to EffectCraft compositions (`Workspaces/_dev/LIVE-LINK.md`).
//!
//! A linked clip is an ordinary file media item: a ProRes 4444 render of an EffectCraft comp (alpha
//! kept), so a FilmCraft without this code, or upstream's, still plays the last render. The link
//! lives in the item's metadata: `ec.project` (the .ecproj), `ec.compId`, `ec.compName`, plus
//! `ec.projectStamp` (the .ecproj's modification time when last rendered) and `ec.version`.
//!
//! Renders go to `<FilmCraft project folder>/EffectCraft Renders/<ecproj name>/<comp>-<id>-v<N>.mov`,
//! never the media cache or the previews folder (both get cleaned). When the .ecproj's modification
//! time changes, a new numbered render is made (Windows may hold the old file open), the clip is
//! switched to it without an undo step or dirtying the project (the title changed, not the edit),
//! and renders older than the one before are deleted. A failed render keeps the last good one and
//! says why. One `decision: link:` line per render and per failure.
//!
//! `link.addEffectCraft` and `link.refreshEffectCraft` render on the spot (CLI, agents, tests);
//! the desktop app checks on a worker thread with its media watch ([`Session::start_link_check`],
//! [`Session::poll_link_check`]).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use filmcraft_project::{ItemId, MediaRef};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

pub const PROJECT: &str = "ec.project";
pub const COMP_ID: &str = "ec.compId";
pub const COMP_NAME: &str = "ec.compName";
pub const STAMP: &str = "ec.projectStamp";
pub const VERSION: &str = "ec.version";

/// The EffectCraft command-line program: `FILMCRAFT_EFFECTCRAFT_CLI`, else `effectcraft-cli` on
/// `PATH`, else `~/Applications/craft/effectcraft-cli` (where the Craft apps are kept on Linux).
pub fn find_cli() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FILMCRAFT_EFFECTCRAFT_CLI").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let exe = if cfg!(windows) { "effectcraft-cli.exe" } else { "effectcraft-cli" };
    if let Some(found) = std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(exe)).find(|p| p.is_file())) {
        return Some(found);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    Some(PathBuf::from(home).join("Applications").join("craft").join(exe)).filter(|p| p.is_file())
}

/// The comp in an .ecproj's text named by `comp` (an id, else a name): (id, name).
pub fn find_comp(ecproj_text: &str, comp: &str) -> Option<(u64, String)> {
    let doc: Value = serde_json::from_str(ecproj_text).ok()?;
    let comps: Vec<(u64, String)> = doc
        .get("items")?
        .as_object()?
        .values()
        .filter(|it| it.pointer("/kind/type").and_then(Value::as_str) == Some("Comp"))
        .filter_map(|it| Some((it.get("id")?.as_u64()?, it.get("name")?.as_str()?.to_string())))
        .collect();
    let by_id = comp.parse::<u64>().ok().and_then(|id| comps.iter().find(|(i, _)| *i == id));
    by_id.or_else(|| comps.iter().find(|(_, n)| n == comp)).cloned()
}

/// The .ecproj's modification time (nanoseconds since 1970), as stored in [`STAMP`].
fn stamp(path: &str) -> Option<String> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(t.as_nanos().to_string())
}

/// A name safe as a file name on every system.
fn safe(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() { "untitled".into() } else { s }
}

/// `<folder of the FilmCraft project>/EffectCraft Renders/<ecproj name>/<comp>-<id>-v<N>.mov`.
pub fn render_path(fc_project: &str, ecproj: &str, comp: (u64, &str), version: u32) -> Option<PathBuf> {
    let folder = Path::new(fc_project).parent()?;
    let stem = Path::new(ecproj).file_stem()?.to_string_lossy();
    Some(folder.join("EffectCraft Renders").join(safe(&stem)).join(format!("{}-{}-v{version}.mov", safe(comp.1), comp.0)))
}

/// One render job: what to run, and what it is for.
#[derive(Clone, Debug)]
pub struct Job {
    pub item: Option<ItemId>,
    pub ecproj: String,
    pub comp: (u64, String),
    pub out: PathBuf,
    pub version: u32,
    pub stamp: Option<String>,
}

/// Run EffectCraft's command line: the comp to a ProRes 4444 movie with alpha. The seconds taken,
/// or why it failed (with the end of what EffectCraft printed, which names its log).
pub fn render(cli: &Path, job: &Job) -> std::result::Result<f64, String> {
    if let Some(dir) = job.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let t0 = Instant::now();
    let out = Command::new(cli)
        .args(["render", "--project", &job.ecproj, "--comp", &job.comp.0.to_string(), "--out"])
        .arg(&job.out)
        .args(["--format", "prores", "--prores", "4444", "--channels", "rgba", "--json"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{}: {e}", cli.display()))?;
    if !out.status.success() || !job.out.is_file() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(format!("EffectCraft's render failed ({}): {}", out.status, tail.join(" / ")));
    }
    Ok(t0.elapsed().as_secs_f64())
}

/// The linked items whose .ecproj changed since their last render: their next jobs.
pub fn due(s: &Session) -> Vec<Job> {
    let Some(fc) = s.path.clone() else { return Vec::new() };
    let mut jobs = Vec::new();
    for (id, it) in &s.project.items {
        let (Some(ecproj), Some(comp_id), Some(name)) = (it.metadata.get(PROJECT), it.metadata.get(COMP_ID), it.metadata.get(COMP_NAME)) else { continue };
        let now = stamp(ecproj);
        if now.is_none() || now.as_ref() == it.metadata.get(STAMP) {
            continue;
        }
        let Ok(cid) = comp_id.parse::<u64>() else { continue };
        let version = it.metadata.get(VERSION).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1) + 1;
        let Some(out) = render_path(&fc, ecproj, (cid, name), version) else { continue };
        jobs.push(Job { item: Some(*id), ecproj: ecproj.clone(), comp: (cid, name.clone()), out, version, stamp: now });
    }
    jobs
}

/// Switch item `id` to a finished render: its file, info and identity, without an undo step or
/// dirtying the project; renders older than the previous one are deleted.
fn swap(s: &mut Session, job: &Job, secs: f64) -> Result<()> {
    let Some(id) = job.item else { return Ok(()) };
    let path = job.out.to_string_lossy().into_owned();
    let src = s.media.open_file(&path, &*s.services).map_err(|e| EngineError::Other(format!("{path}: {e}")))?;
    let info = src.info().clone();
    let identity = crate::relink::identity_of(&*s.services, &path).ok();
    let p = Arc::make_mut(&mut s.project);
    let Some(item) = p.items.get_mut(&id) else { return Ok(()) };
    if let Some(m) = item.as_media_mut() {
        m.media = MediaRef::File { path: path.clone() };
        m.info = info;
        m.identity = identity;
    }
    item.metadata.insert(VERSION.into(), job.version.to_string());
    if let Some(st) = &job.stamp {
        item.metadata.insert(STAMP.into(), st.clone());
    }
    s.media.insert_file(id, &path, src);
    s.bump_view();
    log::info!("decision: link: comp `{}` ({}) of {} rendered in {secs:.1} s to {path}; the clip uses it now", job.comp.1, job.comp.0, job.ecproj);
    // keep this render and the one before (Windows may still hold it open)
    for older in 1..job.version.saturating_sub(1) {
        if let Some(old) = s.path.as_deref().and_then(|fc| render_path(fc, &job.ecproj, (job.comp.0, &job.comp.1), older)) {
            let _ = std::fs::remove_file(old);
        }
    }
    Ok(())
}

fn failed(s: &mut Session, job: &Job, why: &str) {
    log::warn!("decision: link: comp `{}` ({}) of {}: {why}; the clip keeps its last render", job.comp.1, job.comp.0, job.ecproj);
    s.toast(format!("EffectCraft title “{}” not updated: {why}", job.comp.1));
}

/// `link.addEffectCraft {project, comp}`: render that comp, import the render and link it.
pub(crate) fn add(s: &mut Session, p: &Value) -> Result<Value> {
    let ecproj =
        p.get("project").and_then(Value::as_str).ok_or_else(|| EngineError::Other("link.addEffectCraft: need `project` (an .ecproj)".into()))?.to_string();
    let comp = p
        .get("comp")
        .map(|c| c.as_str().map(str::to_string).unwrap_or_else(|| c.to_string()))
        .ok_or_else(|| EngineError::Other("link.addEffectCraft: need `comp` (id or name)".into()))?;
    let fc = s.path.clone().ok_or_else(|| EngineError::Other("save the FilmCraft project first: renders go beside it".into()))?;
    let cli = find_cli().ok_or_else(|| EngineError::Other("EffectCraft's command line was not found: set FILMCRAFT_EFFECTCRAFT_CLI".into()))?;
    let text = std::fs::read_to_string(&ecproj).map_err(|e| EngineError::Other(format!("{ecproj}: {e}")))?;
    let (cid, name) = find_comp(&text, &comp).ok_or_else(|| EngineError::Other(format!("{ecproj} has no composition `{comp}`")))?;
    let out = render_path(&fc, &ecproj, (cid, &name), 1).ok_or_else(|| EngineError::Other("no folder for the render".into()))?;
    let job = Job { item: None, ecproj: ecproj.clone(), comp: (cid, name.clone()), out: out.clone(), version: 1, stamp: stamp(&ecproj) };
    let secs = render(&cli, &job).map_err(EngineError::Other)?;
    let r = s.execute("file.import", json!({"paths": [out.to_string_lossy()]}))?;
    let id = r
        .get("items")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_u64)
        .map(ItemId)
        .ok_or_else(|| EngineError::Other(format!("the render was not imported: {r}")))?;
    let p = Arc::make_mut(&mut s.project);
    if let Some(item) = p.items.get_mut(&id) {
        for (k, v) in [(PROJECT, ecproj.clone()), (COMP_ID, cid.to_string()), (COMP_NAME, name.clone()), (VERSION, "1".into())] {
            item.metadata.insert(k.into(), v);
        }
        if let Some(st) = job.stamp {
            item.metadata.insert(STAMP.into(), st);
        }
    }
    log::info!("decision: link: comp `{name}` ({cid}) of {ecproj} rendered in {secs:.1} s to {}; linked as item {}", out.display(), id.0);
    Ok(json!({"item": id.0, "render": out.to_string_lossy(), "seconds": secs}))
}

/// `link.refreshEffectCraft {}`: render now every linked comp whose project was saved since.
pub(crate) fn refresh(s: &mut Session, _: &Value) -> Result<Value> {
    let jobs = due(s);
    if jobs.is_empty() {
        return Ok(json!({"refreshed": []}));
    }
    let cli = find_cli();
    let mut done = Vec::new();
    for job in jobs {
        let result =
            cli.as_deref().ok_or_else(|| "EffectCraft's command line was not found (FILMCRAFT_EFFECTCRAFT_CLI)".to_string()).and_then(|c| render(c, &job));
        match result {
            Ok(secs) => {
                swap(s, &job, secs)?;
                done.push(job.item.map_or(0, |i| i.0));
            }
            Err(why) => failed(s, &job, &why),
        }
    }
    Ok(json!({"refreshed": done}))
}

/// Finished renders from a worker thread, waiting to be swapped in.
pub type LinkCheck = Arc<Mutex<Option<Vec<(Job, std::result::Result<f64, String>)>>>>;

impl Session {
    /// Desktop app: with the media watch (its timer and window focus), render on a worker thread
    /// the linked comps whose project was saved since, unless a check is still running.
    pub fn start_link_check(&mut self) {
        if self.link_check.is_some() || cfg!(target_arch = "wasm32") {
            return;
        }
        let jobs = due(self);
        if jobs.is_empty() {
            return;
        }
        let slot: LinkCheck = Arc::default();
        let out = slot.clone();
        let cli = find_cli();
        let spawned = std::thread::Builder::new().name("live-link".into()).spawn(move || {
            let results = jobs
                .into_iter()
                .map(|job| {
                    let r = cli
                        .as_deref()
                        .ok_or_else(|| "EffectCraft's command line was not found (FILMCRAFT_EFFECTCRAFT_CLI)".to_string())
                        .and_then(|c| render(c, &job));
                    (job, r)
                })
                .collect();
            *out.lock().unwrap_or_else(PoisonError::into_inner) = Some(results);
        });
        if spawned.is_ok() {
            self.link_check = Some(slot);
        }
    }

    /// Apply a finished check (`None` while there is none, or it is still running).
    pub fn poll_link_check(&mut self) -> Option<Value> {
        let results = self.link_check.as_ref()?.lock().unwrap_or_else(PoisonError::into_inner).take()?;
        self.link_check = None;
        let mut done = Vec::new();
        for (job, r) in results {
            match r.map_err(|e| e.to_string()).and_then(|secs| swap(self, &job, secs).map_err(|e| e.to_string())) {
                Ok(()) => done.push(job.item.map_or(0, |i| i.0)),
                Err(why) => {
                    // the stamp is not taken: the next look tries again only after another save
                    if let Some(id) = job.item
                        && let Some(item) = Arc::make_mut(&mut self.project).items.get_mut(&id)
                        && let Some(st) = &job.stamp
                    {
                        item.metadata.insert(STAMP.into(), st.clone());
                    }
                    failed(self, &job, &why)
                }
            }
        }
        Some(json!({"refreshed": done}))
    }
}

pub fn commands() -> Vec<crate::commands::CommandSpec> {
    use crate::commands::{CommandSpec, always};
    vec![
        CommandSpec {
            id: "link.addEffectCraft",
            label: "Link EffectCraft Composition",
            menu: &[],
            shortcut: None,
            params: r#"{"project":str,"comp":id|name}"#,
            enabled: always,
            run: add,
            journal: true,
        },
        CommandSpec {
            id: "link.refreshEffectCraft",
            label: "Refresh EffectCraft Links",
            menu: &[],
            shortcut: None,
            params: "{}",
            enabled: always,
            run: refresh,
            journal: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ECPROJ: &str = r#"{"savedBy":"0.10.0","items":{"1":{"id":1,"name":"Main Title","kind":{"type":"Comp"}},"2":{"id":2,"name":"Lower Third","kind":{"type":"Comp"}},"3":{"id":3,"name":"Main Title","kind":{"type":"Solid"}}}}"#;

    #[test]
    fn comps_are_found_by_id_then_name() {
        assert_eq!(find_comp(ECPROJ, "2"), Some((2, "Lower Third".into())));
        assert_eq!(find_comp(ECPROJ, "Main Title"), Some((1, "Main Title".into())), "a comp, not the solid of that name");
        assert_eq!(find_comp(ECPROJ, "7"), None);
        assert_eq!(find_comp("not json", "1"), None);
    }

    #[test]
    fn renders_go_beside_the_project_numbered() {
        let p = render_path("/shows/ep1/ep1.fcproj", "/titles/Ep1 Titles.ecproj", (1, "Main/Title?"), 3).unwrap();
        assert_eq!(p, PathBuf::from("/shows/ep1/EffectCraft Renders/Ep1 Titles/Main_Title_-1-v3.mov"));
    }

    #[test]
    fn a_missing_command_line_is_a_clear_failure() {
        let job = Job {
            item: None,
            ecproj: "/x.ecproj".into(),
            comp: (1, "T".into()),
            out: std::env::temp_dir().join("fc-link-none/T.mov"),
            version: 1,
            stamp: None,
        };
        let e = render(Path::new("/nonexistent/effectcraft-cli"), &job).unwrap_err();
        assert!(e.contains("/nonexistent/effectcraft-cli"), "{e}");
    }

    /// End to end with EffectCraft's real command line (`FILMCRAFT_EFFECTCRAFT_CLI`; skipped
    /// without it): link a comp, save the .ecproj again (the clip moves to render 2), then delete
    /// the comp and save (the render fails, the clip keeps render 2).
    #[test]
    fn a_linked_comp_follows_its_project_and_keeps_the_last_render_on_failure() {
        let Some(cli) = std::env::var_os("FILMCRAFT_EFFECTCRAFT_CLI").map(PathBuf::from) else {
            eprintln!("SKIPPED: FILMCRAFT_EFFECTCRAFT_CLI not set");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fc-live-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ecproj = dir.join("titles.ecproj").to_string_lossy().into_owned();
        let ec = |cmds: &[(&str, Value)]| {
            let mut c = Command::new(&cli);
            c.arg("run").env("EFFECTCRAFT_CONFIG_DIR", dir.join("ec-config"));
            if std::path::Path::new(&ecproj).exists() {
                c.args(["--project", &ecproj]);
            } else {
                c.arg("--empty");
            }
            for (id, p) in cmds {
                c.arg(id).arg(p.to_string());
            }
            c.arg("file.save").arg(json!({"path": ecproj}).to_string());
            let out = c.output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        };
        ec(&[("comp.new", json!({"name": "Main Title", "duration": 1.0})), ("comp.new", json!({"name": "Lower Third", "duration": 1.0}))]);
        let mut s = Session::default();
        let fc = dir.join("ep.fcproj").to_string_lossy().into_owned();
        s.execute("file.save", json!({"path": fc})).unwrap();
        let r = s.execute("link.addEffectCraft", json!({"project": ecproj, "comp": "Main Title"})).unwrap();
        let id = ItemId(r["item"].as_u64().unwrap());
        let path = |s: &Session| match &s.project.items[&id].as_media().unwrap().media {
            MediaRef::File { path } => path.clone(),
            _ => String::new(),
        };
        assert!(path(&s).ends_with("EffectCraft Renders/titles/Main Title-1-v1.mov"), "{}", path(&s));
        assert_eq!(s.execute("link.refreshEffectCraft", json!({})).unwrap()["refreshed"], json!([]), "not saved since");
        // saved again (a second later, so the time differs)
        std::thread::sleep(std::time::Duration::from_millis(1100));
        ec(&[("comp.new", json!({"name": "Extra", "duration": 1.0}))]);
        let clean = !s.is_dirty();
        assert_eq!(s.execute("link.refreshEffectCraft", json!({})).unwrap()["refreshed"], json!([id.0]));
        assert!(path(&s).ends_with("Main Title-1-v2.mov"), "{}", path(&s));
        assert!(std::path::Path::new(&path(&s)).is_file());
        assert_eq!(!s.is_dirty(), clean, "the swap is not an edit");
        // the comp deleted: the render fails, the clip keeps render 2
        std::thread::sleep(std::time::Duration::from_millis(1100));
        ec(&[("project.delete", json!({"items": [1]}))]);
        assert_eq!(s.execute("link.refreshEffectCraft", json!({})).unwrap()["refreshed"], json!([]));
        assert!(path(&s).ends_with("Main Title-1-v2.mov"), "{}", path(&s));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
