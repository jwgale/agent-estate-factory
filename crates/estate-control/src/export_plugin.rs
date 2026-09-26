//! `estate pack export-plugin` — Agent Plugin stub from an estate pack.
//!
//! Emits `plugin.json` + `mcp.json` + `skills/*/SKILL.md` (Agent Plugins 1.0
//! floor that Cursor loads). Pack → group metadata, package → skill body
//! stub, routine schedule → commented cron/trigger notes.
//! MCP servers run `estate pack mcp-serve` so member tools call
//! `estate complete` against the source estate. live_sync stays false.
//! Not live Cursor / Grok Bot sync. Not a cron daemon.

use anyhow::{bail, Context, Result};
use estate_schema::{normalize_name, AgentPack, Estate, PackPackage, Routine};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(crate) const EXPORT_SCHEMA: &str = "cell-one.pack-plugin-export.v0";

pub(crate) fn cmd_pack_export_plugin(id: &str, out: &Path, estate_path: &Path) -> Result<()> {
    let estate = estate_schema::load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
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

    let estate_abs = estate_path_for_export(estate_path);

    write_plugin_json(out, pack, orch)?;
    write_mcp_json(out, pack, orch, &estate_abs)?;
    write_skills(out, pack, &packages, &routines)?;
    write_readme(out, pack, orch, &packages, &routines, &estate_abs)?;
    write_mapping_sidecar(out, pack, orch, &packages, &routines, &estate_abs)?;

    println!("pack plugin stub: {}", pack.id);
    println!("  out: {}", out.display());
    println!("  format: agent-plugin (plugin.json + mcp.json + skills/)");
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
    println!("  wired_mcp: yes");
    println!("  live_sync: no");
    Ok(())
}

fn estate_path_for_export(estate_path: &Path) -> String {
    estate_path
        .canonicalize()
        .unwrap_or_else(|_| estate_path.to_path_buf())
        .display()
        .to_string()
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
            "Estate pack {} exported as an Agent Plugin stub. Group members: {members}. Orchestrator: {orch}. MCP tools call estate complete on the source estate. live_sync is false — not live Cursor or Grok Bot sync.",
            pack.id
        ),
        "author": { "name": "cell-one" },
        "keywords": ["cell-one", "estate-pack", pack.id.as_str()],
    });
    write_pretty_json(&out.join("plugin.json"), &body)
}

fn write_mcp_json(out: &Path, pack: &AgentPack, orch: &str, estate_path: &str) -> Result<()> {
    let mut servers = serde_json::Map::new();
    for member in &pack.members {
        let role = crate::pack_mcp::pack_role(Some(orch).filter(|s| *s != "-"), member);
        let env = crate::pack_mcp::export_mcp_env(&pack.id, member, &role, estate_path);
        servers.insert(
            member.clone(),
            json!({
                "type": "stdio",
                "command": "estate",
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

fn skill_markdown(pack: &AgentPack, pkg: &PackPackage, routines: &[&Routine]) -> String {
    let binding = pkg.binding.as_deref().unwrap_or("-");
    let prompt = pkg.prompt.as_deref().unwrap_or("-");
    let note = pkg.note.as_deref().unwrap_or("-");
    let desc = if let Some(n) = pkg.note.as_deref().filter(|s| !s.is_empty()) {
        format!("{n} Scaffold only — not live Cursor/Grok Bot sync.")
    } else {
        format!(
            "Pack package {} on {}. Binding {binding}. Scaffold only — not live Cursor/Grok Bot sync.",
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
         Run on the estate (not a live Cursor/Grok Bot install):\n\
         \n\
             estate package run --id {id} --estate <estate.yaml> --mock\n\
         \n\
         Pack member MCP tools call `estate complete` via `estate pack mcp-serve`.\n\
         This skill body stays a stub. live_sync is false.\n\
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
) -> Result<()> {
    let mut md = String::new();
    md.push_str(&format!("# {} plugin stub\n\n", pack.id));
    md.push_str("Agent Plugin scaffold from an estate pack. Cursor can load this\n");
    md.push_str("layout (`plugin.json` + `mcp.json` + `skills/`).\n\n");
    md.push_str("**Bridge, not live Cursor / Grok Bot sync.** MCP servers run\n");
    md.push_str("`estate pack mcp-serve` so member tools call `estate complete`\n");
    md.push_str("against the source estate. `estate` must be on PATH.\n");
    md.push_str("`live_sync: false`. `wired_mcp: true`. Routines stay comments.\n");
    md.push_str("This directory does not install a Cursor/Grok Bot plugin, does\n");
    md.push_str("not start a cron daemon, and does not rank mixed-select.\n\n");
    md.push_str(&format!("Estate file: `{estate_path}`\n\n"));
    md.push_str("## Mapping\n\n");
    md.push_str("| Estate | Plugin stub |\n");
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
    md.push_str("One stdio server per pack member. `command` is `estate`;\n");
    md.push_str("`args` are `pack mcp-serve`. Each process exposes tool\n");
    md.push_str("`complete`, which runs:\n\n");
    md.push_str("    estate complete --agent <member> --pack <pack-id> \\\n");
    md.push_str("      --estate $CELL_ESTATE_PATH --prompt <tool input>\n\n");
    md.push_str("Env carries `CELL_ESTATE_PACK`, `CELL_ESTATE_MEMBER`,\n");
    md.push_str("`CELL_ESTATE_ROLE`, and `CELL_ESTATE_PATH`. Non-orchestrator\n");
    md.push_str("members refuse `refuse:pack-orchestrator` the same as\n");
    md.push_str("`estate complete --pack`. Pass `mock: true` on the tool to\n");
    md.push_str("use in-process drivers. Not a live Cursor/Grok Bot install.\n\n");
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
                "# {} → package {}\n# schedule: {sched}\n# suggested cron/trigger: {}\n# live-local wake: estate routine tick --id {}\n",
                r.id,
                r.package,
                cron_comment(sched),
                r.id
            ));
        }
    }
    md.push_str("```\n");
    md.push_str("\n`estate routine tick` remains the local wake. This file does\n");
    md.push_str("not install a Cursor or Grok Bot trigger.\n");
    fs::write(out.join("README.md"), md)?;
    Ok(())
}

fn write_mapping_sidecar(
    out: &Path,
    pack: &AgentPack,
    orch: &str,
    packages: &[&PackPackage],
    routines: &[&Routine],
    estate_path: &str,
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
        "live_sync": false,
        "wired_mcp": true,
        "mcp": {
            "command": "estate",
            "args": ["pack", "mcp-serve"],
            "tool": "complete",
            "invokes": "estate complete --agent <member> --pack <pack-id>",
        },
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
    use super::cron_comment;

    #[test]
    fn cron_comment_maps_shorthands() {
        assert_eq!(cron_comment("@hourly"), "0 * * * *");
        assert_eq!(cron_comment("@daily"), "0 0 * * *");
        assert_eq!(cron_comment("0 6 * * *"), "0 6 * * *");
        assert_eq!(cron_comment("@every 1h"), "@every 1h");
        assert_eq!(cron_comment(""), "(none)");
    }
}
