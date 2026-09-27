//! `estate pack plugin-install-local` — copy a proved Agent Plugin into
//! `~/.cursor/plugins/local/<plugin-name>` as a real directory.
//!
//! Runs the same asserts as `estate pack plugin-prove` (baked absolute
//! estate bin, horizon mock receipt, research `refuse:pack-orchestrator`,
//! non-thin `RUNNER.md` / `SESSION.md`), then copies files. A symlink whose
//! target is outside the local plugins folder is `refuse:plugin-install-symlink`.
//! A foreign directory without `.estate-pack-install.json` refuses unless
//! `--force`. When `@anysphere/cursor-plugins` `loadUserLocalPlugins` is
//! available, the report records the names it returns. Otherwise
//! `loaded: skipped:loader-unavailable`. That does not claim Cursor Customize loaded the plugin.
//! `live_sync` stays false. `READY_FOR_LIVE_TEST`: no.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::export_plugin::cmd_pack_export_plugin;
use crate::pack_mcp::COMPLETE_TIMEOUT_ENV;
use crate::plugin_prove::{print_report, prove_exported_plugin, ProveReport};

pub(crate) const INSTALL_SCHEMA: &str = "cell-one.pack-plugin-install.v0";
pub(crate) const MARKER_FILE: &str = ".estate-pack-install.json";
const PLUGIN_SCHEMA_ID: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
const MCP_SCHEMA_ID: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
const MANAGED_BY: &str = "estate";

const PLUGIN_KEYS: &[&str] = &[
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];
const AUTHOR_KEYS: &[&str] = &["name", "email", "url"];
const STDIO_KEYS: &[&str] = &["type", "command", "args", "env", "cwd"];

const LOADER_JS: &str = r#"
const spec = process.env.CELL_CURSOR_PLUGINS_MODULE || "@anysphere/cursor-plugins";
let mod = null;
try { mod = require(spec); } catch (e) { mod = null; }
function namesFrom(result) {
  const list = Array.isArray(result) ? result : (result && (result.plugins || result.names)) || [];
  const names = [];
  for (const item of list) {
    if (typeof item === "string") names.push(item);
    else if (item && typeof item.name === "string") names.push(item.name);
    else if (item && typeof item.id === "string") names.push(item.id);
  }
  return names;
}
(async () => {
  const fn = mod && (mod.loadUserLocalPlugins || (mod.default && mod.default.loadUserLocalPlugins));
  if (typeof fn !== "function") {
    process.stdout.write(JSON.stringify({ available: false }));
    return;
  }
  const root = process.env.CELL_CURSOR_PLUGINS_LOCAL || "";
  try {
    const result = await fn({ pluginsDir: root, dir: root, localRoot: root, root });
    process.stdout.write(JSON.stringify({ available: true, names: namesFrom(result) }));
  } catch (err) {
    process.stdout.write(JSON.stringify({
      available: true,
      error: String(err && err.message || err)
    }));
  }
})();
"#;

#[derive(Debug)]
enum DestAction {
    Create,
    Unlink,
    ReplaceDir,
}

struct InstallFacts {
    plugin_name: String,
    skills: Vec<String>,
    mcp_servers: Vec<String>,
    plugin_schema: String,
    mcp_schema: String,
    estate_path: String,
    export_schema: String,
}

pub(crate) fn cmd_pack_plugin_install_local(
    id: &str,
    out: Option<&Path>,
    target: Option<&Path>,
    estate_path: &Path,
    prompt: &str,
    complete_timeout_secs: Option<u64>,
    check_only: bool,
    force: bool,
) -> Result<()> {
    let local_root = match target {
        Some(path) => path.to_path_buf(),
        None => default_plugins_local_root()?,
    };
    let timeout = crate::pack_mcp::resolve_complete_timeout(
        complete_timeout_secs,
        std::env::var(COMPLETE_TIMEOUT_ENV).ok(),
    )?;
    let out_dir = match out {
        Some(path) => path.to_path_buf(),
        None if check_only => {
            bail!("refuse:plugin-install-out: --check-only requires --out");
        }
        None => throwaway_out(id),
    };

    if !check_only {
        cmd_pack_export_plugin(id, &out_dir, estate_path, Some(timeout.as_secs()))?;
    } else if !out_dir.is_dir() {
        bail!(
            "refuse:plugin-install-out: --out '{}' is not a directory",
            out_dir.display()
        );
    }

    let prove = prove_exported_plugin(&out_dir, prompt)?;
    print_report(&prove);
    if !prove.ok {
        let detail = prove
            .checks
            .iter()
            .find(|c| !c.ok)
            .map(|c| c.detail.as_str())
            .unwrap_or("one or more checks failed");
        bail!("refuse:plugin-install: prove failed: {detail}");
    }

    let facts = read_install_facts(&out_dir)?;
    let dest = local_root.join(&facts.plugin_name);
    if same_path(&out_dir, &dest)? {
        bail!(
            "refuse:plugin-install-target: export out and install path are the same ({})",
            dest.display()
        );
    }

    fs::create_dir_all(&local_root).with_context(|| {
        format!(
            "refuse:plugin-install-target: create {}",
            local_root.display()
        )
    })?;
    let action = dest_action(&dest, &local_root, force)?;

    let staging = local_root.join(format!(
        ".estate-pack-staging-{}-{}",
        facts.plugin_name,
        unique_nanos()
    ));
    let copied = (|| -> Result<()> {
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        copy_plugin_tree(&out_dir, &staging)?;
        Ok(())
    })();
    if let Err(err) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }

    let installed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let tip = git_tip();
    let mut marker = marker_value(
        &prove,
        &facts,
        &dest,
        &out_dir,
        &tip,
        &installed_at,
        "skipped:loader-pending",
        &[],
    );
    if let Err(err) = write_marker(&staging, &marker) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }

    if let Err(err) = swap_in(&dest, &staging, action) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }

    let (loaded, loader_names) = probe_loader(&local_root, &facts.plugin_name);
    marker["loaded"] = json!(loaded);
    marker["loader_names"] = json!(loader_names);
    if let Err(err) = write_marker(&dest, &marker) {
        return Err(err);
    }

    print_install_report(
        &prove,
        &facts,
        &dest,
        &out_dir,
        &loaded,
        &loader_names,
        &tip,
    );
    Ok(())
}

fn default_plugins_local_root() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:plugin-install-home: HOME is unset; pass --target or set HOME")
        })?;
    Ok(PathBuf::from(home)
        .join(".cursor")
        .join("plugins")
        .join("local"))
}

fn throwaway_out(id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cell-pack-plugin-install-{id}-{}", unique_nanos()))
}

fn unique_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn read_install_facts(out: &Path) -> Result<InstallFacts> {
    let plugin = read_json(&out.join("plugin.json")).with_context(|| {
        format!(
            "refuse:plugin-install-manifest: read {}",
            out.join("plugin.json").display()
        )
    })?;
    let mcp = read_json(&out.join("mcp.json")).with_context(|| {
        format!(
            "refuse:plugin-install-manifest: read {}",
            out.join("mcp.json").display()
        )
    })?;
    let plugin_schema = validate_plugin_json(&plugin)?;
    let (mcp_schema, mcp_servers) = validate_mcp_json(&mcp)?;
    let plugin_name = plugin
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if Path::new(&plugin_name).components().count() != 1 {
        bail!(
            "refuse:plugin-install-manifest: plugin name '{plugin_name}' is not a single path component"
        );
    }
    let mapping = read_json(&out.join("estate-pack.json")).unwrap_or_else(|_| json!({}));
    Ok(InstallFacts {
        plugin_name,
        skills: list_skills(out)?,
        mcp_servers,
        plugin_schema,
        mcp_schema,
        estate_path: mapping
            .get("estate_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        export_schema: mapping
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn list_skills(out: &Path) -> Result<Vec<String>> {
    let skills = out.join("skills");
    if !skills.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for ent in fs::read_dir(&skills).with_context(|| format!("read {}", skills.display()))? {
        let ent = ent?;
        if ent.file_type()?.is_dir() && ent.path().join("SKILL.md").is_file() {
            names.push(ent.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn validate_plugin_json(plugin: &Value) -> Result<String> {
    let obj = plugin.as_object().ok_or_else(|| {
        anyhow::anyhow!("refuse:plugin-install-manifest: plugin.json must be an object")
    })?;
    let schema = obj.get("$schema").and_then(Value::as_str).unwrap_or("");
    let full = schema == PLUGIN_SCHEMA_ID;
    if full {
        for key in obj.keys() {
            if !PLUGIN_KEYS.contains(&key.as_str()) {
                bail!(
                    "refuse:plugin-install-manifest: plugin.json field '{key}' is not in the Agent Plugins 1.0 schema"
                );
            }
        }
    }
    let name = obj.get("name").and_then(Value::as_str).ok_or_else(|| {
        anyhow::anyhow!("refuse:plugin-install-manifest: plugin.json name must be a string")
    })?;
    if !agent_plugin_name_ok(name) {
        bail!(
            "refuse:plugin-install-manifest: plugin.json name '{name}' does not match the Agent Plugins name pattern"
        );
    }
    if let Some(version) = obj.get("version") {
        if !version.is_string() {
            bail!("refuse:plugin-install-manifest: plugin.json version must be a string");
        }
    }
    if let Some(description) = obj.get("description") {
        if !description.is_string() {
            bail!("refuse:plugin-install-manifest: plugin.json description must be a string");
        }
    }
    if let Some(author) = obj.get("author") {
        let author_obj = author.as_object().ok_or_else(|| {
            anyhow::anyhow!("refuse:plugin-install-manifest: plugin.json author must be an object")
        })?;
        if full {
            for key in author_obj.keys() {
                if !AUTHOR_KEYS.contains(&key.as_str()) {
                    bail!(
                        "refuse:plugin-install-manifest: plugin.json author.{key} is not in the schema"
                    );
                }
            }
        }
        for key in AUTHOR_KEYS {
            if let Some(val) = author_obj.get(*key) {
                if !val.is_string() {
                    bail!(
                        "refuse:plugin-install-manifest: plugin.json author.{key} must be a string"
                    );
                }
            }
        }
    }
    if let Some(keywords) = obj.get("keywords") {
        let items = keywords.as_array().ok_or_else(|| {
            anyhow::anyhow!("refuse:plugin-install-manifest: plugin.json keywords must be an array")
        })?;
        if items.iter().any(|item| !item.is_string()) {
            bail!("refuse:plugin-install-manifest: plugin.json keywords must be strings");
        }
    }
    if full {
        Ok("agent-plugins-1.0".into())
    } else {
        Ok("structural".into())
    }
}

fn validate_mcp_json(mcp: &Value) -> Result<(String, Vec<String>)> {
    let obj = mcp.as_object().ok_or_else(|| {
        anyhow::anyhow!("refuse:plugin-install-manifest: mcp.json must be an object")
    })?;
    let schema = obj.get("$schema").and_then(Value::as_str).unwrap_or("");
    let full = schema == MCP_SCHEMA_ID;
    if full {
        for key in obj.keys() {
            if key != "$schema" && key != "mcpServers" {
                bail!(
                    "refuse:plugin-install-manifest: mcp.json field '{key}' is not in the Agent Plugins 1.0 schema"
                );
            }
        }
    }
    let servers = obj
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!("refuse:plugin-install-manifest: mcp.json missing mcpServers")
        })?;
    if servers.is_empty() {
        bail!("refuse:plugin-install-manifest: mcp.json mcpServers is empty");
    }
    let mut names = Vec::new();
    for (name, server) in servers {
        if name.is_empty() {
            bail!("refuse:plugin-install-manifest: mcp server id is empty");
        }
        validate_mcp_server(name, server, full)?;
        names.push(name.clone());
    }
    names.sort();
    if full {
        Ok(("agent-plugins-1.0".into(), names))
    } else {
        Ok(("structural".into(), names))
    }
}

fn validate_mcp_server(name: &str, server: &Value, full: bool) -> Result<()> {
    let obj = server.as_object().ok_or_else(|| {
        anyhow::anyhow!("refuse:plugin-install-manifest: mcp server '{name}' must be an object")
    })?;
    if full {
        for key in obj.keys() {
            if !STDIO_KEYS.contains(&key.as_str()) {
                bail!(
                    "refuse:plugin-install-manifest: mcp server '{name}' field '{key}' is not in the schema"
                );
            }
        }
        let typ = obj.get("type").and_then(Value::as_str).unwrap_or("");
        if typ != "stdio" {
            bail!(
                "refuse:plugin-install-manifest: mcp server '{name}' type must be stdio for this install"
            );
        }
    }
    let command = obj.get("command").and_then(Value::as_str).unwrap_or("");
    if command.is_empty() {
        bail!("refuse:plugin-install-manifest: mcp server '{name}' missing command");
    }
    if let Some(args) = obj.get("args") {
        let items = args.as_array().ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:plugin-install-manifest: mcp server '{name}' args must be an array"
            )
        })?;
        if items.iter().any(|item| !item.is_string()) {
            bail!("refuse:plugin-install-manifest: mcp server '{name}' args must be strings");
        }
    }
    if let Some(env) = obj.get("env") {
        let env_obj = env.as_object().ok_or_else(|| {
            anyhow::anyhow!(
                "refuse:plugin-install-manifest: mcp server '{name}' env must be an object"
            )
        })?;
        for (key, val) in env_obj {
            if key == "PLUGIN_ROOT" || key == "PLUGIN_DATA" {
                bail!("refuse:plugin-install-manifest: mcp server '{name}' env must not set {key}");
            }
            if !val.is_string() {
                bail!(
                    "refuse:plugin-install-manifest: mcp server '{name}' env.{key} must be a string"
                );
            }
        }
    }
    Ok(())
}

/// Agent Plugins 1.0 `name` pattern:
/// `^(?!.*(?:--|\.\.))[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$` and length 1..=64.
pub(crate) fn agent_plugin_name_ok(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if name.contains("--") || name.contains("..") {
        return false;
    }
    let alnum = |c: u8| c.is_ascii_digit() || c.is_ascii_lowercase();
    if !alnum(bytes[0]) || !alnum(bytes[bytes.len() - 1]) {
        return false;
    }
    bytes.iter().all(|c| alnum(*c) || *c == b'.' || *c == b'-')
}

fn dest_action(dest: &Path, local_root: &Path, force: bool) -> Result<DestAction> {
    let meta = match fs::symlink_metadata(dest) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(DestAction::Create),
        Err(err) => {
            bail!(
                "refuse:plugin-install-target: cannot stat {}: {err}",
                dest.display()
            )
        }
    };
    if meta.file_type().is_symlink() {
        refuse_if_symlink_outside(dest, local_root)?;
    }
    if !estate_managed(dest) && !force {
        bail!(
            "refuse:plugin-install-foreign: {} has no estate install marker ({MARKER_FILE}). Pass --force to overwrite a foreign install",
            dest.display()
        );
    }
    if meta.file_type().is_symlink() {
        Ok(DestAction::Unlink)
    } else if meta.is_dir() {
        Ok(DestAction::ReplaceDir)
    } else {
        bail!(
            "refuse:plugin-install-target: {} exists and is not a directory",
            dest.display()
        )
    }
}

fn refuse_if_symlink_outside(link: &Path, root: &Path) -> Result<()> {
    let raw = fs::read_link(link)
        .with_context(|| format!("refuse:plugin-install-symlink: readlink {}", link.display()))?;
    let joined = if raw.is_absolute() {
        raw
    } else {
        link.parent().unwrap_or(root).join(raw)
    };
    let canon = joined.canonicalize().map_err(|err| {
        anyhow::anyhow!(
            "refuse:plugin-install-symlink: {} target '{}' does not resolve inside {}: {err}",
            link.display(),
            joined.display(),
            root.display()
        )
    })?;
    let root_c = root.canonicalize().with_context(|| {
        format!(
            "refuse:plugin-install-target: canonicalize {}",
            root.display()
        )
    })?;
    if !canon.starts_with(&root_c) {
        bail!(
            "refuse:plugin-install-symlink: {} points at {} which is outside {}",
            link.display(),
            canon.display(),
            root_c.display()
        );
    }
    Ok(())
}

fn estate_managed(dest: &Path) -> bool {
    let path = if dest.join(MARKER_FILE).is_file() {
        dest.join(MARKER_FILE)
    } else {
        return false;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    is_estate_marker(&value)
}

pub(crate) fn is_estate_marker(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(INSTALL_SCHEMA)
        && value.get("managed_by").and_then(Value::as_str) == Some(MANAGED_BY)
}

fn swap_in(dest: &Path, staging: &Path, action: DestAction) -> Result<()> {
    match action {
        DestAction::Create => {}
        DestAction::Unlink => {
            fs::remove_file(dest).with_context(|| format!("unlink {}", dest.display()))?;
        }
        DestAction::ReplaceDir => {
            fs::remove_dir_all(dest).with_context(|| format!("remove {}", dest.display()))?;
        }
    }
    if fs::rename(staging, dest).is_err() {
        copy_plugin_tree(staging, dest)?;
        fs::remove_dir_all(staging)?;
    }
    Ok(())
}

fn copy_plugin_tree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;
    let mut stack = Vec::new();
    copy_dir(src, src, dst, &mut stack)
}

fn copy_dir(root: &Path, src: &Path, dst: &Path, stack: &mut Vec<PathBuf>) -> Result<()> {
    let canon = src
        .canonicalize()
        .with_context(|| format!("refuse:plugin-install-copy: canonicalize {}", src.display()))?;
    if stack.iter().any(|seen| seen == &canon) {
        bail!("refuse:plugin-install-symlink: cycle at {}", src.display());
    }
    stack.push(canon);
    for ent in fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let ent = ent?;
        let path = ent.path();
        let name = ent.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "prove-state" || name_str == MARKER_FILE {
            continue;
        }
        let dest = dst.join(&name);
        let ft = ent.file_type()?;
        if ft.is_symlink() {
            let target = symlink_inside(root, &path)?;
            let meta = fs::metadata(&target).with_context(|| {
                format!("refuse:plugin-install-symlink: stat {}", path.display())
            })?;
            if meta.is_dir() {
                fs::create_dir_all(&dest)?;
                copy_dir(root, &path, &dest, stack)?;
            } else if meta.is_file() {
                fs::copy(&target, &dest).with_context(|| format!("copy {}", path.display()))?;
            } else {
                bail!(
                    "refuse:plugin-install-copy: unsupported symlink {}",
                    path.display()
                );
            }
        } else if ft.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir(root, &path, &dest, stack)?;
        } else if ft.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &dest).with_context(|| format!("copy {}", path.display()))?;
        } else {
            bail!(
                "refuse:plugin-install-copy: unsupported file {}",
                path.display()
            );
        }
    }
    stack.pop();
    Ok(())
}

fn symlink_inside(root: &Path, link: &Path) -> Result<PathBuf> {
    let raw = fs::read_link(link)
        .with_context(|| format!("refuse:plugin-install-symlink: readlink {}", link.display()))?;
    let joined = if raw.is_absolute() {
        raw
    } else {
        link.parent().unwrap_or(root).join(raw)
    };
    let canon = joined.canonicalize().map_err(|err| {
        anyhow::anyhow!(
            "refuse:plugin-install-symlink: {} target '{}' does not resolve inside the export: {err}",
            link.display(),
            joined.display()
        )
    })?;
    let root_c = root.canonicalize()?;
    if !canon.starts_with(&root_c) {
        bail!(
            "refuse:plugin-install-symlink: {} points at {} which is outside the export",
            link.display(),
            canon.display()
        );
    }
    Ok(canon)
}

fn same_path(a: &Path, b: &Path) -> Result<bool> {
    if a == b {
        return Ok(true);
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => Ok(left == right),
        _ => Ok(false),
    }
}

fn marker_value(
    prove: &ProveReport,
    facts: &InstallFacts,
    dest: &Path,
    out: &Path,
    tip: &str,
    installed_at: &str,
    loaded: &str,
    loader_names: &[String],
) -> Value {
    json!({
        "schema": INSTALL_SCHEMA,
        "managed_by": MANAGED_BY,
        "pack_id": prove.pack_id,
        "plugin_name": facts.plugin_name,
        "install_path": dest.display().to_string(),
        "export_out": out.display().to_string(),
        "export_schema": facts.export_schema,
        "estate_bin": prove.estate_bin,
        "estate_path": facts.estate_path,
        "tip": tip,
        "installed_at": installed_at,
        "skills": facts.skills,
        "mcp_servers": facts.mcp_servers,
        "runner_docs": if prove.runner_docs { "yes" } else { "no" },
        "session_docs": if prove.session_docs { "yes" } else { "no" },
        "loaded": loaded,
        "loader_names": loader_names,
        "live_sync": false,
        "ready_for_live_test": false,
    })
}

fn write_marker(dir: &Path, marker: &Value) -> Result<()> {
    let path = dir.join(MARKER_FILE);
    let body = serde_json::to_string_pretty(marker)?;
    fs::write(&path, format!("{body}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn git_tip() -> String {
    let Ok(out) = Command::new("git").args(["rev-parse", "HEAD"]).output() else {
        return "unknown".into();
    };
    if !out.status.success() {
        return "unknown".into();
    }
    let tip = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if tip.len() == 40 && tip.chars().all(|c| c.is_ascii_hexdigit()) {
        tip
    } else {
        "unknown".into()
    }
}

fn probe_loader(local_root: &Path, plugin_name: &str) -> (String, Vec<String>) {
    let mut cmd = Command::new("node");
    cmd.arg("-e")
        .arg(LOADER_JS)
        .env("CELL_CURSOR_PLUGINS_LOCAL", local_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => return ("skipped:loader-unavailable".into(), Vec::new()),
    };
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > Duration::from_secs(5) => {
                let _ = child.kill();
                let _ = child.wait();
                return ("skipped:loader-timeout".into(), Vec::new());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(_) => return ("skipped:loader-error".into(), Vec::new()),
        }
    };
    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    if !status.success() && stdout.trim().is_empty() {
        return ("skipped:loader-unavailable".into(), Vec::new());
    }
    let parsed: Value = match serde_json::from_str(stdout.trim()) {
        Ok(value) => value,
        Err(_) => return ("skipped:loader-error".into(), Vec::new()),
    };
    if parsed.get("available") != Some(&Value::Bool(true)) {
        return ("skipped:loader-unavailable".into(), Vec::new());
    }
    let names = parsed
        .get("names")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if parsed.get("error").and_then(Value::as_str).is_some() {
        return ("skipped:loader-error".into(), names);
    }
    if names.iter().any(|name| name == plugin_name) {
        ("yes".into(), names)
    } else {
        ("skipped:not-listed".into(), names)
    }
}

fn print_install_report(
    prove: &ProveReport,
    facts: &InstallFacts,
    dest: &Path,
    out: &Path,
    loaded: &str,
    loader_names: &[String],
    tip: &str,
) {
    let skills = if facts.skills.is_empty() {
        "-".to_string()
    } else {
        facts.skills.join(", ")
    };
    let servers = if facts.mcp_servers.is_empty() {
        "-".to_string()
    } else {
        facts.mcp_servers.join(", ")
    };
    let names = if loader_names.is_empty() {
        "-".to_string()
    } else {
        loader_names.join(", ")
    };
    println!("pack plugin-install-local: {}", prove.pack_id);
    println!("  ok: yes");
    println!("  install_path: {}", dest.display());
    println!("  plugin_name: {}", facts.plugin_name);
    println!("  skills: {skills}");
    println!("  mcp_servers: {servers}");
    println!(
        "  runner_docs: {}",
        if prove.runner_docs { "yes" } else { "no" }
    );
    println!(
        "  session_docs: {}",
        if prove.session_docs { "yes" } else { "no" }
    );
    println!("  loaded: {loaded}");
    println!("  loader_names: {names}");
    println!("  estate_bin: {}", prove.estate_bin);
    println!("  tip: {tip}");
    println!("  plugin_schema: {}", facts.plugin_schema);
    println!("  mcp_schema: {}", facts.mcp_schema);
    println!("  export_out: {}", out.display());
    println!("  live_sync: no");
    println!("  READY_FOR_LIVE_TEST: no");
    let report = json!({
        "schema": INSTALL_SCHEMA,
        "ok": true,
        "pack_id": prove.pack_id,
        "plugin_name": facts.plugin_name,
        "install_path": dest.display().to_string(),
        "export_out": out.display().to_string(),
        "skills": facts.skills,
        "mcp_servers": facts.mcp_servers,
        "runner_docs": if prove.runner_docs { "yes" } else { "no" },
        "session_docs": if prove.session_docs { "yes" } else { "no" },
        "loaded": loaded,
        "loader_names": loader_names,
        "estate_bin": prove.estate_bin,
        "tip": tip,
        "plugin_schema": facts.plugin_schema,
        "mcp_schema": facts.mcp_schema,
        "prove_ok": prove.ok,
        "live_sync": false,
        "ready_for_live_test": false,
    });
    println!("{report}");
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_plugin_name_accepts_kebab_and_rejects_uppercase() {
        assert!(agent_plugin_name_ok("research-crew"));
        assert!(agent_plugin_name_ok("a"));
        assert!(agent_plugin_name_ok("a.b"));
        assert!(!agent_plugin_name_ok("Research"));
        assert!(!agent_plugin_name_ok("research--crew"));
        assert!(!agent_plugin_name_ok("a."));
        assert!(!agent_plugin_name_ok("../x"));
        assert!(!agent_plugin_name_ok(""));
    }

    #[test]
    fn estate_marker_requires_schema_and_managed_by() {
        assert!(is_estate_marker(&json!({
            "schema": INSTALL_SCHEMA,
            "managed_by": "estate"
        })));
        assert!(!is_estate_marker(&json!({
            "schema": INSTALL_SCHEMA,
            "managed_by": "other"
        })));
        assert!(!is_estate_marker(&json!({"name": "research-crew"})));
    }

    #[cfg(unix)]
    #[test]
    fn copy_refuses_symlink_outside_export() {
        let nanos = unique_nanos();
        let root = std::env::temp_dir().join(format!("cell-install-copy-{nanos}"));
        let outside = std::env::temp_dir().join(format!("cell-install-outside-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "nope").unwrap();
        fs::write(root.join("plugin.json"), "{}\n").unwrap();
        std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("leak")).unwrap();
        let dst = std::env::temp_dir().join(format!("cell-install-dst-{nanos}"));
        let _ = fs::remove_dir_all(&dst);
        let err = copy_plugin_tree(&root, &dst).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-install-symlink"), "{err}");
        assert!(!dst.join("leak").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        let _ = fs::remove_dir_all(&dst);
    }

    #[test]
    fn validate_plugin_json_accepts_export_shape() {
        let plugin = json!({
            "$schema": PLUGIN_SCHEMA_ID,
            "name": "research-crew",
            "version": "0.0.0",
            "description": "Estate pack",
            "author": { "name": "cell-one" },
            "keywords": ["cell-one", "estate-pack"]
        });
        assert_eq!(validate_plugin_json(&plugin).unwrap(), "agent-plugins-1.0");
        let mut extra = plugin.clone();
        extra["displayName"] = json!("Nope");
        let err = validate_plugin_json(&extra).unwrap_err().to_string();
        assert!(err.contains("refuse:plugin-install-manifest"), "{err}");
    }
}
