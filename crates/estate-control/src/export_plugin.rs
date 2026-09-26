//! `estate pack export-plugin` — Agent Plugin export from an estate pack.
//!
//! Emits `plugin.json` + `mcp.json` + `skills/*/SKILL.md` + `INSTALL.md`
//! (Agent Plugins 1.0 floor that Cursor loads). Pack → group metadata,
//! package → skill body that instructs calling the wired member MCP tool
//! `complete` with the package prompt (mock/live notes as appropriate),
//! routine schedule → commented cron/trigger notes. MCP `command` is the
//! absolute `estate` binary resolved from `current_exe` at export time;
//! args stay `pack mcp-serve` so member tools call `estate complete`
//! against the source estate. Env writes `CELL_MCP_COMPLETE_TIMEOUT_SECS`
//! so Cursor-spawned MCP inherits the same cap as CLI prove. Export
//! refuses (`refuse:export-estate-bin`) when that binary cannot be
//! resolved — no silent PATH name `estate`. live_sync stays false. Not
//! live Cursor / Grok Bot sync. Not a cron daemon. When MCP is wired
//! and skills are non-stub, labels say pack plugin / Agent Plugin
//! export — not "pack plugin stub".

use anyhow::{bail, Context, Result};
use estate_schema::{normalize_name, AgentPack, Estate, PackPackage, Routine};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(crate) const EXPORT_SCHEMA: &str = "cell-one.pack-plugin-export.v0";

pub(crate) fn cmd_pack_export_plugin(
    id: &str,
    out: &Path,
    estate_path: &Path,
    complete_timeout_secs: Option<u64>,
) -> Result<()> {
    let timeout = crate::pack_mcp::resolve_complete_timeout(
        complete_timeout_secs,
        std::env::var(crate::pack_mcp::COMPLETE_TIMEOUT_ENV).ok(),
    )?;
    let timeout_secs = timeout.as_secs();
    let estate_abs = estate_path_for_export(estate_path)?;
    let estate_bin = estate_bin_for_export()?;
    let estate = estate_schema::load_estate(Path::new(&estate_abs))
        .with_context(|| format!("load {estate_abs}"))?;
    let pack = estate
        .pack(id)
        .ok_or_else(|| anyhow::anyhow!("refuse:unknown-pack: pack '{id}' not on estate"))?;
    if out.exists() && !out.is_dir() {
        bail!(
            "refuse:export-plugin-out: --out '{}' exists and is not a directory",
            out.display()
        );
    }
    fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;

    let packages = packages_for_pack(&estate, pack);
    let routines = routines_for_packages(&estate, &packages);
    let orch = pack.orchestrator.as_deref().unwrap_or("-");

    write_plugin_json(out, pack, orch)?;
    write_mcp_json(out, pack, orch, &estate_abs, &estate_bin, timeout_secs)?;
    write_skills(out, pack, &packages, &routines)?;
    write_readme(
        out,
        pack,
        orch,
        &packages,
        &routines,
        &estate_abs,
        &estate_bin,
    )?;
    write_install_md(out, pack, orch)?;
    write_mapping_sidecar(
        out,
        pack,
        orch,
        &packages,
        &routines,
        &estate_abs,
        &estate_bin,
        timeout_secs,
    )?;

    println!("pack plugin: {}", pack.id);
    println!("  out: {}", out.display());
    println!("  format: agent-plugin (plugin.json + mcp.json + skills/ + INSTALL.md)");
    println!("  members: {}", pack.members.join(", "));
    println!("  orchestrator: {orch}");
    if packages.is_empty() {
        println!("  skills: -");
    } else {
        println!(
            "  skills: {}",
            packages
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if routines.is_empty() {
        println!("  routines (commented): -");
    } else {
        let notes: Vec<String> = routines
            .iter()
            .map(|r| {
                format!(
                    "{} {}",
                    r.id,
                    r.schedule.as_deref().unwrap_or("(no schedule)")
                )
            })
            .collect();
        println!("  routines (commented): {}", notes.join(", "));
    }
    println!("  estate_bin: {estate_bin}");
    println!("  complete_timeout_secs: {timeout_secs}");
    println!("  wired_mcp: yes");
    println!("  live_sync: no");
    Ok(())
}

/// Absolute estate path for exported `CELL_ESTATE_PATH`.
/// Refuses when canonicalize fails or the result is not absolute.
/// Never silently embeds a relative path.
pub(crate) fn estate_path_for_export(estate_path: &Path) -> Result<String> {
    let canonical = estate_path.canonicalize().map_err(|err| {
        anyhow::anyhow!(
            "refuse:export-estate-path: cannot canonicalize '{}': {err}",
            estate_path.display()
        )
    })?;
    if !canonical.is_absolute() {
        bail!(
            "refuse:export-estate-path: refused relative estate path '{}'",
            canonical.display()
        );
    }
    Ok(canonical.display().to_string())
}

/// Absolute `estate` binary for exported `mcp.json` `command`.
/// Resolves `std::env::current_exe` and canonicalizes it at export time.
/// Refuses when current_exe or canonicalize fails, the result is not a
/// file, or the result is not absolute. Never silently embeds `estate`.
pub(crate) fn estate_bin_for_export() -> Result<String> {
    let exe = std::env::current_exe().map_err(|err| {
        anyhow::anyhow!("refuse:export-estate-bin: cannot resolve current estate binary: {err}")
    })?;
    resolve_estate_bin_path(&exe)
}

/// Canonicalize a candidate estate binary. Shared so tests can refuse
/// missing/relative paths without stubbing `current_exe`.
pub(crate) fn resolve_estate_bin_path(path: &Path) -> Result<String> {
    let canonical = path.canonicalize().map_err(|err| {
        anyhow::anyhow!(
            "refuse:export-estate-bin: cannot resolve estate binary '{}': {err}",
            path.display()
        )
    })?;
    if !canonical.is_file() {
        bail!(
            "refuse:export-estate-bin: estate binary '{}' is not a file",
            canonical.display()
        );
    }
    if !canonical.is_absolute() {
        bail!(
            "refuse:export-estate-bin: refused relative estate binary '{}'",
            canonical.display()
        );
    }
    Ok(canonical.display().to_string())
}

fn packages_for_pack<'a>(estate: &'a Estate, pack: &AgentPack) -> Vec<&'a PackPackage> {
    let want = normalize_name(&pack.id);
    let mut pkgs: Vec<&PackPackage> = estate
        .pack_packages
        .iter()
        .filter(|p| normalize_name(&p.pack) == want)
        .collect();
    pkgs.sort_by(|a, b| a.id.cmp(&b.id));
    pkgs
}

fn routines_for_packages<'a>(estate: &'a Estate, packages: &[&PackPackage]) -> Vec<&'a Routine> {
    let ids: Vec<String> = packages.iter().map(|p| normalize_name(&p.id)).collect();
    let mut rows: Vec<&Routine> = estate
        .routines
        .iter()
        .filter(|r| ids.contains(&normalize_name(&r.package)))
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

fn write_plugin_json(out: &Path, pack: &AgentPack, orch: &str) -> Result<()> {
    let members = pack.members.join(", ");
    let body = json!({
        "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
        "name": pack.id,
        "version": "0.0.0",
        "description": format!(
            "Estate pack {} exported as an Agent Plugin. Group members: {members}. Orchestrator: {orch}. Skill bodies call the wired member MCP tool complete with the package prompt. MCP tools call estate complete on the source estate. live_sync is false — not live Cursor or Grok Bot sync.",
            pack.id
        ),
        "author": { "name": "cell-one" },
        "keywords": ["cell-one", "estate-pack", pack.id.as_str()],
    });
    write_pretty_json(&out.join("plugin.json"), &body)
}

fn write_mcp_json(
    out: &Path,
    pack: &AgentPack,
    orch: &str,
    estate_path: &str,
    estate_bin: &str,
    complete_timeout_secs: u64,
) -> Result<()> {
    let mut servers = serde_json::Map::new();
    for member in &pack.members {
        let role = crate::pack_mcp::pack_role(Some(orch).filter(|s| *s != "-"), member);
        let env = crate::pack_mcp::export_mcp_env(
            &pack.id,
            member,
            &role,
            estate_path,
            complete_timeout_secs,
        );
        servers.insert(
            member.clone(),
            json!({
                "type": "stdio",
                "command": estate_bin,
                "args": ["pack", "mcp-serve"],
                "env": env,
            }),
        );
    }
    let body = json!({
        "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
        "mcpServers": Value::Object(servers),
    });
    write_pretty_json(&out.join("mcp.json"), &body)
}

fn write_skills(
    out: &Path,
    pack: &AgentPack,
    packages: &[&PackPackage],
    routines: &[&Routine],
) -> Result<()> {
    let skills = out.join("skills");
    if packages.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(&skills)?;
    for pkg in packages {
        let dir = skills.join(&pkg.id);
        fs::create_dir_all(&dir)?;
        let related: Vec<&Routine> = routines
            .iter()
            .copied()
            .filter(|r| normalize_name(&r.package) == normalize_name(&pkg.id))
            .collect();
        fs::write(dir.join("SKILL.md"), skill_markdown(pack, pkg, &related))?;
    }
    Ok(())
}

fn skill_complete_member(pack: &AgentPack) -> &str {
    pack.orchestrator
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "-")
        .or_else(|| pack.members.first().map(String::as_str))
        .unwrap_or("-")
}

/// Single-line MCP `complete` arguments for the package prompt.
/// Mock includes `"mock": true`. Binding becomes `object` when present.
fn complete_tool_args_line(pkg: &PackPackage, mock: bool) -> String {
    let prompt = pkg
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("<set prompt>");
    let prompt_json = serde_json::to_string(prompt).unwrap_or_else(|_| "\"<set prompt>\"".into());
    let mut parts = vec![format!("\"prompt\": {prompt_json}")];
    if mock {
        parts.push("\"mock\": true".into());
    }
    if let Some(binding) = pkg
        .binding
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let binding_json = serde_json::to_string(binding).unwrap_or_else(|_| "\"\"".into());
        parts.push(format!("\"object\": {binding_json}"));
    }
    format!("{{ {} }}", parts.join(", "))
}

fn skill_markdown(pack: &AgentPack, pkg: &PackPackage, routines: &[&Routine]) -> String {
    let binding = pkg.binding.as_deref().unwrap_or("-");
    let prompt = pkg.prompt.as_deref().unwrap_or("-");
    let note = pkg.note.as_deref().unwrap_or("-");
    let caller = skill_complete_member(pack);
    let desc = if let Some(n) = pkg.note.as_deref().filter(|s| !s.is_empty()) {
        format!(
            "{n} Call the wired member MCP tool complete with the package prompt. live_sync is false — not live Cursor/Grok Bot sync."
        )
    } else {
        format!(
            "Pack package {} on {}. Binding {binding}. Call the wired member MCP tool complete with the package prompt. live_sync is false — not live Cursor/Grok Bot sync.",
            pkg.id, pack.id
        )
    };
    let mut chain = String::new();
    if pkg.chain.is_empty() {
        chain.push_str("-");
    } else {
        let hops: Vec<String> = pkg
            .chain
            .iter()
            .map(|h| format!("{} -> {}", h.agent, h.binding))
            .collect();
        chain = hops.join("; ");
    }
    let mock_args = complete_tool_args_line(pkg, true);
    let live_args = complete_tool_args_line(pkg, false);
    let mut body = format!(
        "---\n\
         name: {id}\n\
         description: {desc}\n\
         ---\n\
         \n\
         # {id}\n\
         \n\
         Estate pack package (skill analog) on pack `{pack}`.\n\
         \n\
         - pack: {pack}\n\
         - binding: {binding}\n\
         - prompt: {prompt}\n\
         - chain: {chain}\n\
         - note: {note}\n\
         \n\
         Call the wired pack member MCP tool `complete` with this package prompt.\n\
         Speak it on member `{caller}` (`mcp.json` server). Non-orchestrator members\n\
         refuse `refuse:pack-orchestrator` the same as `estate complete --pack`.\n\
         The server is `estate pack mcp-serve`; tool `complete` runs\n\
         `estate complete --agent {caller} --pack {pack}` against `CELL_ESTATE_PATH`.\n\
         Optional `session` / `session_id` is pack-scoped multi-hop memory;\n\
         `session_create: true` mints a new id. `live_sync` stays false.\n\
         \n\
         Mock (in-process drivers, no live generate):\n\
         \n\
             {mock_args}\n\
         \n\
         Live (omit `mock` when `CELL_LOCAL_ENDPOINT` / `XAI_API_KEY` are set).\n\
         Receipts stay on the source estate. Not a live PASS:\n\
         \n\
             {live_args}\n\
         \n\
         `live_sync: false`. Not live Cursor / Grok Bot sync. Routines stay comments.\n\
         \n\
         Estate-side equivalent (not required for the plugin skill):\n\
         \n\
             estate package run --id {id} --estate <estate.yaml> --mock\n\
         \n",
        id = pkg.id,
        pack = pack.id,
    );
    body.push_str("<!-- routine schedule notes (not a cron daemon, not live sync):\n");
    if routines.is_empty() {
        body.push_str("  (no standing routines declare this package)\n");
    } else {
        for r in routines {
            let sched = r.schedule.as_deref().unwrap_or("-");
            body.push_str(&format!(
                "  {}: package={} schedule={sched}\n  # suggested Cursor/Grok trigger (comment only): {}\n",
                r.id,
                r.package,
                cron_comment(sched)
            ));
        }
    }
    body.push_str("-->\n");
    body
}

fn write_readme(
    out: &Path,
    pack: &AgentPack,
    orch: &str,
    packages: &[&PackPackage],
    routines: &[&Routine],
    estate_path: &str,
    estate_bin: &str,
) -> Result<()> {
    let mut md = String::new();
    md.push_str(&format!("# {} pack plugin\n\n", pack.id));
    md.push_str("Agent Plugin export from an estate pack. Cursor can load this\n");
    md.push_str("layout (`plugin.json` + `mcp.json` + `skills/` + `INSTALL.md`).\n\n");
    md.push_str("**Bridge, not live Cursor / Grok Bot sync.** MCP servers run\n");
    md.push_str("`estate pack mcp-serve` so member tools call `estate complete`\n");
    md.push_str("against the source estate. Skill bodies instruct calling the\n");
    md.push_str("wired member MCP tool `complete` with the package prompt\n");
    md.push_str("(mock / live notes as appropriate). They are not stubs.\n");
    md.push_str("Exported `mcp.json` `command` is the absolute estate binary\n");
    md.push_str("resolved at export time — Cursor does not need `estate` on PATH.\n");
    md.push_str("`live_sync: false`. `wired_mcp: true`. Routines stay comments.\n");
    md.push_str("This directory does not install a Cursor/Grok Bot plugin, does\n");
    md.push_str("not start a cron daemon, and does not rank mixed-select.\n\n");
    md.push_str(&format!("Estate file: `{estate_path}`\n"));
    md.push_str(&format!("Estate binary: `{estate_bin}`\n\n"));
    md.push_str("## Mapping\n\n");
    md.push_str("| Estate | Agent Plugin |\n");
    md.push_str("| --- | --- |\n");
    md.push_str(&format!(
        "| pack `{}` (members {}, orchestrator {orch}) | `plugin.json` group metadata |\n",
        pack.id,
        pack.members.join(", ")
    ));
    if packages.is_empty() {
        md.push_str("| (no pack_packages) | (no skills/) |\n");
    } else {
        for pkg in packages {
            md.push_str(&format!(
                "| package `{}` | `skills/{}/SKILL.md` |\n",
                pkg.id, pkg.id
            ));
        }
    }
    if routines.is_empty() {
        md.push_str("| (no routines on these packages) | (no cron comments) |\n");
    } else {
        for r in routines {
            let sched = r.schedule.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| routine `{}` schedule `{sched}` | commented trigger `{}` |\n",
                r.id,
                cron_comment(sched)
            ));
        }
    }
    md.push_str("\n## MCP (wired to estate complete)\n\n");
    md.push_str("One stdio server per pack member. `command` is the absolute\n");
    md.push_str("estate binary resolved at export (`current_exe` + canonicalize);\n");
    md.push_str("`args` are `pack mcp-serve`. Each process exposes tool\n");
    md.push_str("`complete`, which runs:\n\n");
    md.push_str("    estate complete --agent <member> --pack <pack-id> \\\n");
    md.push_str("      --estate $CELL_ESTATE_PATH --prompt <tool input>\n\n");
    md.push_str("Env carries `CELL_ESTATE_PACK`, `CELL_ESTATE_MEMBER`,\n");
    md.push_str("`CELL_ESTATE_ROLE`, `CELL_ESTATE_PATH`, and\n");
    md.push_str("`CELL_MCP_COMPLETE_TIMEOUT_SECS`. Non-orchestrator\n");
    md.push_str("members refuse `refuse:pack-orchestrator` the same as\n");
    md.push_str("`estate complete --pack`. Pass `mock: true` on the tool to\n");
    md.push_str("use in-process drivers. Optional `session` / `session_id`\n");
    md.push_str("carries pack-scoped multi-hop memory. Not a live Cursor/Grok Bot install.\n\n");
    for member in &pack.members {
        let role = crate::pack_mcp::pack_role(Some(orch).filter(|s| *s != "-"), member);
        md.push_str(&format!("- `{member}` ({role})\n"));
    }
    md.push_str("\n## Routine schedule notes (comment only)\n\n");
    md.push_str("```\n");
    if routines.is_empty() {
        md.push_str("# (none)\n");
    } else {
        for r in routines {
            let sched = r.schedule.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "# {} → package {}\n# schedule: {sched}\n# suggested cron/trigger: {}\n# live-local wake: estate routine tick --id {}\n# standing loop: estate routine watch --id {}\n",
                r.id,
                r.package,
                cron_comment(sched),
                r.id,
                r.id
            ));
        }
    }
    md.push_str("```\n");
    md.push_str("\n`estate routine tick` remains the local wake.\n");
    md.push_str("`estate routine watch` is the local operator loop (tick + digest).\n");
    md.push_str("This file does not install a Cursor or Grok Bot trigger.\n");
    md.push_str("Human Cursor smoke steps: `INSTALL.md`.\n");
    md.push_str("CLI gate before install: `estate pack plugin-prove`.\n");
    fs::write(out.join("README.md"), md)?;
    Ok(())
}

/// Literal human install loop. Mock / Cursor smoke only.
fn write_install_md(out: &Path, pack: &AgentPack, orch: &str) -> Result<()> {
    let caller = skill_complete_member(pack);
    let member = pack
        .members
        .iter()
        .map(String::as_str)
        .find(|m| *m != caller)
        .unwrap_or("research");
    let orch_name = if orch == "-" { caller } else { orch };
    let md = format!(
        "# INSTALL\n\
         \n\
         Human Cursor smoke for this Agent Plugin export. Mock only.\n\
         Pack `{pack}`. Orchestrator `{orch}`.\n\
         \n\
         ## 1. Plugin folder\n\
         \n\
         This directory (the export root). It holds `plugin.json`, `mcp.json`,\n\
         `skills/`, `INSTALL.md`, and `estate-pack.json`.\n\
         \n\
         ## 2. Load `mcp.json` in Cursor\n\
         \n\
         Cursor Settings → MCP / Agent Plugins → add this folder, or load the\n\
         servers from `mcp.json`. Each `mcpServers` key is one pack member.\n\
         `command` is the absolute `estate` binary baked at export. Do not\n\
         replace it with bare `estate`. Do not require `estate` on PATH.\n\
         \n\
         ## 3. Horizon `complete` sample\n\
         \n\
         On server `{caller}` (orchestrator), call tool `complete` with:\n\
         \n\
             {{ \"prompt\": \"ping\", \"mock\": true }}\n\
         \n\
         ## 4. Expected receipt\n\
         \n\
         `isError` false. Text includes a decision receipt:\n\
         \n\
             decision receipt: …\n\
             schema: cell-one.decision-receipt.v0\n\
             surface: complete\n\
             pack_id: {pack}\n\
             handoff_from: {caller}\n\
         \n\
         Mock is ok. This is not a live generate.\n\
         \n\
         ## 5. Research refuse\n\
         \n\
         On server `{member}`, same payload.\n\
         \n\
         Expected: `isError` true and `refuse:pack-orchestrator`.\n\
         \n\
         ## 6. READY_FOR_LIVE_TEST: no\n\
         \n\
         Mock / human Cursor smoke only. `live_sync: false`. Not live Grok Bot\n\
         sync. Not a live PASS. No secrets in this file.\n\
         \n\
         Optional (cheap): pack-scoped session two-hop on `{caller}` with\n\
         `session` / `session_create` — hop 2 should see hop 1 (`context=applied`).\n\
         Not required to call this export ready for Cursor smoke.\n\
         \n\
         CLI gate (same checks, no Cursor UI):\n\
         \n\
             estate pack plugin-prove --id {pack} --estate <estate.yaml> --out <this-dir>\n\
         \n",
        pack = pack.id,
        orch = orch_name,
        caller = caller,
        member = member,
    );
    fs::write(out.join("INSTALL.md"), md)?;
    Ok(())
}

fn write_mapping_sidecar(
    out: &Path,
    pack: &AgentPack,
    orch: &str,
    packages: &[&PackPackage],
    routines: &[&Routine],
    estate_path: &str,
    estate_bin: &str,
    complete_timeout_secs: u64,
) -> Result<()> {
    let orch_val = if orch == "-" {
        Value::Null
    } else {
        json!(orch)
    };
    let pkgs: Vec<Value> = packages
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "skill": format!("skills/{}/SKILL.md", p.id),
                "binding": p.binding,
                "prompt": p.prompt,
                "mcp_tool": "complete",
                "skill_stub": false,
            })
        })
        .collect();
    let rows: Vec<Value> = routines
        .iter()
        .map(|r| {
            let sched = r.schedule.as_deref();
            json!({
                "id": r.id,
                "package": r.package,
                "schedule": sched,
                "cron_note": sched.map(cron_comment),
                "live_trigger": false,
            })
        })
        .collect();
    let body = json!({
        "schema": EXPORT_SCHEMA,
        "pack_id": pack.id,
        "group": {
            "id": pack.id,
            "members": pack.members,
            "orchestrator": orch_val,
        },
        "packages": pkgs,
        "routines": rows,
        "estate_path": estate_path,
        "estate_bin": estate_bin,
        "live_sync": false,
        "wired_mcp": true,
        "mcp": {
            "command": estate_bin,
            "args": ["pack", "mcp-serve"],
            "tool": "complete",
            "invokes": "estate complete --agent <member> --pack <pack-id>",
            "complete_timeout_secs": complete_timeout_secs,
            "complete_timeout_env": "CELL_MCP_COMPLETE_TIMEOUT_SECS",
        },
        "install": "INSTALL.md",
    });
    write_pretty_json(&out.join("estate-pack.json"), &body)
}

/// Comment-only cron/trigger hint. Not a daemon and not a live install.
pub(crate) fn cron_comment(schedule: &str) -> String {
    let raw = schedule.trim();
    if raw.is_empty() || raw == "-" {
        return "(none)".into();
    }
    if raw.eq_ignore_ascii_case("@hourly") {
        return "0 * * * *".into();
    }
    if raw.eq_ignore_ascii_case("@daily") {
        return "0 0 * * *".into();
    }
    raw.into()
}

fn write_pretty_json(path: &Path, value: &Value) -> Result<()> {
    let body = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{body}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        complete_tool_args_line, cron_comment, estate_bin_for_export, estate_path_for_export,
        resolve_estate_bin_path, skill_complete_member, skill_markdown,
    };
    use estate_schema::{AgentPack, PackPackage};
    use std::path::{Path, PathBuf};

    fn sample_pack() -> AgentPack {
        AgentPack {
            id: "research-crew".into(),
            members: vec!["horizon".into(), "research".into()],
            orchestrator: Some("horizon".into()),
            chain: vec![],
        }
    }

    fn sample_pkg() -> PackPackage {
        PackPackage {
            id: "classify-ping".into(),
            pack: "research-crew".into(),
            prompt: Some("ping".into()),
            binding: Some("ag_news".into()),
            note: Some("Skill analog".into()),
            chain: vec![],
        }
    }

    #[test]
    fn cron_comment_maps_shorthands() {
        assert_eq!(cron_comment("@hourly"), "0 * * * *");
        assert_eq!(cron_comment("@daily"), "0 0 * * *");
        assert_eq!(cron_comment("0 6 * * *"), "0 6 * * *");
        assert_eq!(cron_comment("@every 1h"), "@every 1h");
        assert_eq!(cron_comment(""), "(none)");
    }

    #[test]
    fn estate_path_for_export_canonicalizes_existing_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-export-estate-path-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("estate.yaml");
        std::fs::write(&file, "agents: []\n").unwrap();
        let abs = estate_path_for_export(&file).unwrap();
        assert!(Path::new(&abs).is_absolute(), "{abs}");
        assert!(abs.ends_with("estate.yaml"), "{abs}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn estate_path_for_export_refuses_missing_relative_path() {
        let rel = PathBuf::from("no-such-cell-estate-for-export.yaml");
        assert!(!rel.is_absolute());
        let err = estate_path_for_export(&rel).unwrap_err().to_string();
        assert!(err.contains("refuse:export-estate-path"), "{err}");
        assert!(err.contains("cannot canonicalize"), "{err}");
    }

    #[test]
    fn estate_bin_for_export_resolves_current_exe() {
        let abs = estate_bin_for_export().unwrap();
        let path = Path::new(&abs);
        assert!(path.is_absolute(), "{abs}");
        assert!(path.is_file(), "{abs}");
    }

    #[test]
    fn resolve_estate_bin_path_canonicalizes_existing_file() {
        let exe = std::env::current_exe().unwrap();
        let abs = resolve_estate_bin_path(&exe).unwrap();
        assert!(Path::new(&abs).is_absolute(), "{abs}");
        assert!(Path::new(&abs).is_file(), "{abs}");
    }

    #[test]
    fn resolve_estate_bin_path_refuses_missing_relative_path() {
        let rel = PathBuf::from("no-such-cell-estate-bin-for-export");
        assert!(!rel.is_absolute());
        let err = resolve_estate_bin_path(&rel).unwrap_err().to_string();
        assert!(err.contains("refuse:export-estate-bin"), "{err}");
        assert!(err.contains("cannot resolve estate binary"), "{err}");
    }

    #[test]
    fn resolve_estate_bin_path_refuses_directory() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-export-estate-bin-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let err = resolve_estate_bin_path(&dir).unwrap_err().to_string();
        assert!(err.contains("refuse:export-estate-bin"), "{err}");
        assert!(err.contains("is not a file"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn complete_tool_args_line_includes_prompt_mock_and_object() {
        let pkg = sample_pkg();
        assert_eq!(
            complete_tool_args_line(&pkg, true),
            r#"{ "prompt": "ping", "mock": true, "object": "ag_news" }"#
        );
        assert_eq!(
            complete_tool_args_line(&pkg, false),
            r#"{ "prompt": "ping", "object": "ag_news" }"#
        );
    }

    #[test]
    fn skill_markdown_instructs_mcp_complete_not_stub() {
        let pack = sample_pack();
        let pkg = sample_pkg();
        let md = skill_markdown(&pack, &pkg, &[]);
        assert_eq!(skill_complete_member(&pack), "horizon");
        assert!(md.contains("Call the wired pack member MCP tool `complete`"), "{md}");
        assert!(md.contains("estate pack mcp-serve"), "{md}");
        assert!(
            md.contains(r#"{ "prompt": "ping", "mock": true, "object": "ag_news" }"#),
            "{md}"
        );
        assert!(md.contains("live_sync: false"), "{md}");
        assert!(md.contains("no standing routines"), "{md}");
        assert!(!md.contains("stays a stub"), "{md}");
        assert!(!md.contains("body stub"), "{md}");
        assert!(!md.contains("Scaffold only"), "{md}");
    }
}
