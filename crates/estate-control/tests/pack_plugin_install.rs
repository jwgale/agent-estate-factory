//! `estate pack plugin-install-local` — proved export copied into
//! `$HOME/.cursor/plugins/local/<plugin-name>`.
//!
//! Fixture: examples/fixtures/agent-pack-handoff.yaml.
//! Happy path overrides HOME and XDG so the real home is untouched.
//! Refuses thin docs, a bare estate bin, an outside symlink, and a
//! foreign directory. Records `loaded: yes` only when a loader module
//! lists the plugin. live_sync stays false. Does not invent a live PASS
//! or claim Cursor Customize loaded the plugin.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cell-pack-plugin-install-{name}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], home: &std::path::Path, extra: &[(&str, &str)]) -> (bool, String, String) {
    let xdg = home.join("xdg");
    std::fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_estate"));
    cmd.args(args)
        .current_dir(repo_root())
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("XDG_DATA_HOME", &xdg)
        .env_remove("XAI_API_KEY")
        .env_remove("CELL_FRONTIER_ENDPOINT")
        .env_remove("CELL_LOCAL_ENDPOINT")
        .env_remove("CELL_FRONTIER_MODEL")
        .env_remove("CELL_LOCAL_LIVE")
        .env_remove("CELL_CURSOR_PLUGINS_MODULE");
    for (key, value) in extra {
        cmd.env(key, value);
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn assert_locked_cksum() {
    let sum = Command::new("cksum")
        .arg(repo_root().join("examples/estate.yaml"))
        .output()
        .unwrap();
    let sum_text = String::from_utf8_lossy(&sum.stdout);
    assert!(
        sum_text.starts_with("43770130 3391"),
        "examples/estate.yaml cksum changed: {sum_text}"
    );
}

fn fixture() -> PathBuf {
    repo_root().join("examples/fixtures/agent-pack-handoff.yaml")
}

fn last_json_object(stdout: &str) -> Value {
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON report in:\n{stdout}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("parse {line}: {e}"))
}

fn assert_real_dir(dir: &std::path::Path) {
    let meta = std::fs::symlink_metadata(dir).unwrap();
    assert!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "{} should be a real directory",
        dir.display()
    );
}

fn fake_loader(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("fake-cursor-plugins.js");
    std::fs::write(
        &path,
        r#"
const fs = require("fs");
const path = require("path");
async function loadUserLocalPlugins(opts) {
  const root = opts.pluginsDir || opts.localRoot || opts.dir || opts.root;
  const names = [];
  if (!root || !fs.existsSync(root)) return [];
  for (const ent of fs.readdirSync(root, { withFileTypes: true })) {
    if (!ent.isDirectory() || ent.name.startsWith(".")) continue;
    const manifest = path.join(root, ent.name, "plugin.json");
    if (!fs.existsSync(manifest)) continue;
    const body = JSON.parse(fs.readFileSync(manifest, "utf8"));
    names.push({ name: body.name || ent.name });
  }
  return names;
}
module.exports = { loadUserLocalPlugins };
"#,
    )
    .unwrap();
    path
}

#[test]
fn docs_document_plugin_install_local_loop() {
    let root = repo_root();
    let north = std::fs::read_to_string(root.join("docs/NORTH-STAR.md")).unwrap();
    let day = std::fs::read_to_string(root.join("docs/OPERATOR-DAY.md")).unwrap();
    let lang = std::fs::read_to_string(root.join("docs/UBIQUITOUS_LANGUAGE.md")).unwrap();
    let log = std::fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    for (name, text) in [
        ("NORTH-STAR", north.as_str()),
        ("OPERATOR-DAY", day.as_str()),
        ("UBIQUITOUS_LANGUAGE", lang.as_str()),
        ("CHANGELOG", log.as_str()),
    ] {
        assert!(
            text.contains("plugin-install-local"),
            "{name} missing plugin-install-local"
        );
        assert!(
            text.contains("READY_FOR_LIVE_TEST"),
            "{name} missing READY_FOR_LIVE_TEST"
        );
    }
    assert!(day.contains("## 3h2. Pack plugin-install-local"), "{day}");
    assert!(day.contains("refuse:plugin-install-symlink"), "{day}");
    assert!(day.contains(".estate-pack-install.json"), "{day}");
    assert!(day.contains("loaded: skipped:loader-unavailable"), "{day}");
    assert!(
        north.contains("loaded: skipped:loader-unavailable"),
        "{north}"
    );
    assert!(log.contains("43770130 3391"), "{log}");
    assert!(
        log.contains("live_sync: false")
            || log.contains("`live_sync` stays false")
            || log.contains("live_sync` stays false"),
        "{log}"
    );
}

#[test]
fn plugin_install_refuses_when_home_unset() {
    let dir = scratch("nohome");
    let estate = fixture().display().to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_estate"));
    cmd.args([
        "pack",
        "plugin-install-local",
        "--id",
        "research-crew",
        "--estate",
        &estate,
        "--out",
        &dir.join("unused").display().to_string(),
    ])
    .current_dir(repo_root())
    .env_remove("HOME")
    .env("XDG_CONFIG_HOME", &dir)
    .env("XDG_DATA_HOME", &dir)
    .env_remove("CELL_CURSOR_PLUGINS_MODULE");
    let out = cmd.output().unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{combined}");
    assert!(
        combined.contains("refuse:plugin-install-home"),
        "{combined}"
    );
}

#[test]
fn plugin_install_happy_path_under_temp_home_then_loader() {
    assert_locked_cksum();
    let home = scratch("home");
    let estate = fixture().display().to_string();
    let expected_bin = std::fs::canonicalize(env!("CARGO_BIN_EXE_estate"))
        .unwrap()
        .display()
        .to_string();

    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "plugin-install-local",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--prompt",
            "ping",
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.contains("pack plugin-prove: research-crew"),
        "{stdout}"
    );
    assert!(
        stdout.contains("pack plugin-install-local: research-crew"),
        "{stdout}"
    );
    assert!(stdout.contains("horizon_complete: ok"), "{stdout}");
    assert!(stdout.contains("research_complete: ok"), "{stdout}");
    assert!(stdout.contains("runner_docs: yes"), "{stdout}");
    assert!(stdout.contains("session_docs: yes"), "{stdout}");
    assert!(
        stdout.contains("loaded: skipped:loader-unavailable"),
        "{stdout}"
    );
    assert!(!stdout.contains("loaded: yes"), "{stdout}");
    assert!(stdout.contains("READY_FOR_LIVE_TEST: no"), "{stdout}");
    assert!(stdout.contains("live_sync: no"), "{stdout}");
    assert!(!stdout.contains("live PASS"), "{stdout}");
    assert!(!stdout.contains("Customize PASS"), "{stdout}");
    assert!(!stderr.contains("live PASS"), "{stderr}");
    assert!(stdout.contains(&expected_bin), "{stdout}");

    let report = last_json_object(&stdout);
    assert_eq!(report["schema"], "cell-one.pack-plugin-install.v0");
    assert_eq!(report["ok"], true);
    assert_eq!(report["pack_id"], "research-crew");
    assert_eq!(report["plugin_name"], "research-crew");
    assert_eq!(report["loaded"], "skipped:loader-unavailable");
    assert_eq!(report["runner_docs"], "yes");
    assert_eq!(report["session_docs"], "yes");
    assert_eq!(report["live_sync"], false);
    assert_eq!(report["ready_for_live_test"], false);
    assert_eq!(report["prove_ok"], true);
    assert_eq!(report["estate_bin"], expected_bin);
    assert_eq!(report["plugin_schema"], "agent-plugins-1.0");
    assert_eq!(report["mcp_schema"], "agent-plugins-1.0");
    let skills = report["skills"].as_array().unwrap();
    let skill_names: Vec<&str> = skills.iter().filter_map(Value::as_str).collect();
    assert!(skill_names.contains(&"classify-once"), "{report}");
    assert!(skill_names.contains(&"classify-ping"), "{report}");
    let servers = report["mcp_servers"].as_array().unwrap();
    let server_names: Vec<&str> = servers.iter().filter_map(Value::as_str).collect();
    assert!(server_names.contains(&"horizon"), "{report}");
    assert!(server_names.contains(&"research"), "{report}");

    let install = home
        .join(".cursor")
        .join("plugins")
        .join("local")
        .join("research-crew");
    assert_eq!(report["install_path"], install.display().to_string());
    assert_real_dir(&install);
    assert!(install.join("plugin.json").is_file());
    assert!(install.join("mcp.json").is_file());
    assert!(install.join("RUNNER.md").is_file());
    assert!(install.join("SESSION.md").is_file());
    assert!(install.join("skills/classify-ping/SKILL.md").is_file());
    assert!(!install.join("prove-state").exists());
    let install_md = std::fs::read_to_string(install.join("INSTALL.md")).unwrap();
    assert!(install_md.contains("plugin-install-local"), "{install_md}");
    assert!(
        install_md.contains("loaded: skipped:loader-unavailable"),
        "{install_md}"
    );

    let marker: Value = serde_json::from_str(
        &std::fs::read_to_string(install.join(".estate-pack-install.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(marker["schema"], "cell-one.pack-plugin-install.v0");
    assert_eq!(marker["managed_by"], "estate");
    assert_eq!(marker["pack_id"], "research-crew");
    assert_eq!(marker["plugin_name"], "research-crew");
    assert_eq!(marker["estate_bin"], expected_bin);
    assert_eq!(marker["live_sync"], false);
    assert_eq!(marker["ready_for_live_test"], false);
    assert_eq!(marker["loaded"], "skipped:loader-unavailable");
    let tip = marker["tip"].as_str().unwrap();
    assert!(
        tip == "unknown" || (tip.len() == 40 && tip.chars().all(|c| c.is_ascii_hexdigit())),
        "{tip}"
    );

    let xdg = home.join("xdg");
    let xdg_entries: Vec<_> = std::fs::read_dir(&xdg).unwrap().collect();
    assert!(
        xdg_entries.is_empty(),
        "XDG dir should stay empty: {xdg_entries:?}"
    );

    let export_out = report["export_out"].as_str().unwrap().to_string();
    let loader = fake_loader(&home);
    let loader_s = loader.display().to_string();
    let (ok2, stdout2, stderr2) = run(
        &[
            "pack",
            "plugin-install-local",
            "--check-only",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &export_out,
        ],
        &home,
        &[("CELL_CURSOR_PLUGINS_MODULE", &loader_s)],
    );
    assert!(ok2, "stderr={stderr2}\nstdout={stdout2}");
    assert!(stdout2.contains("loaded: yes"), "{stdout2}");
    assert!(stdout2.contains("loader_names: research-crew"), "{stdout2}");
    assert!(!stdout2.contains("Customize PASS"), "{stdout2}");
    assert!(!stdout2.contains("live PASS"), "{stdout2}");
    let report2 = last_json_object(&stdout2);
    assert_eq!(report2["loaded"], "yes");
    assert_eq!(report2["loader_names"][0], "research-crew");
    assert_real_dir(&install);
    let marker2: Value = serde_json::from_str(
        &std::fs::read_to_string(install.join(".estate-pack-install.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(marker2["loaded"], "yes");
    assert_eq!(marker2["loader_names"][0], "research-crew");
    assert_eq!(marker2["managed_by"], "estate");
    assert_locked_cksum();
}

#[test]
fn plugin_install_refuses_foreign_unless_force() {
    assert_locked_cksum();
    let home = scratch("foreign");
    let out = home.join("export");
    let target = home.join("plugins-local");
    let estate = fixture().display().to_string();
    let out_s = out.display().to_string();
    let target_s = target.display().to_string();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "export-plugin",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    let dest = target.join("research-crew");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("FOREIGN.txt"), "keep-me").unwrap();

    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "plugin-install-local",
            "--check-only",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
            "--target",
            &target_s,
        ],
        &home,
        &[],
    );
    let combined = format!("{stdout}{stderr}");
    assert!(!ok, "{combined}");
    assert!(
        combined.contains("refuse:plugin-install-foreign"),
        "{combined}"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("FOREIGN.txt")).unwrap(),
        "keep-me"
    );
    assert!(!dest.join(".estate-pack-install.json").exists());

    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "plugin-install-local",
            "--check-only",
            "--force",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
            "--target",
            &target_s,
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    assert_real_dir(&dest);
    assert!(dest.join("plugin.json").is_file());
    assert!(dest.join(".estate-pack-install.json").is_file());
    assert!(!dest.join("FOREIGN.txt").exists());
    assert_locked_cksum();
}

#[test]
fn plugin_install_refuses_symlink_outside_target_even_with_force() {
    assert_locked_cksum();
    let home = scratch("symlink");
    let out = home.join("export");
    let target = home.join("plugins-local");
    let outside = home.join("outside");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("sentinel.txt"), "untouched").unwrap();
    let dest = target.join("research-crew");
    std::os::unix::fs::symlink(&outside, &dest).unwrap();

    let estate = fixture().display().to_string();
    let out_s = out.display().to_string();
    let target_s = target.display().to_string();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "export-plugin",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");

    for force in [false, true] {
        let mut args = vec![
            "pack",
            "plugin-install-local",
            "--check-only",
            "--id",
            "research-crew",
            "--estate",
            estate.as_str(),
            "--out",
            out_s.as_str(),
            "--target",
            target_s.as_str(),
        ];
        if force {
            args.push("--force");
        }
        let (ok, stdout, stderr) = run(&args, &home, &[]);
        let combined = format!("{stdout}{stderr}");
        assert!(!ok, "force={force} {combined}");
        assert!(
            combined.contains("refuse:plugin-install-symlink"),
            "force={force} {combined}"
        );
    }
    assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        std::fs::read_to_string(outside.join("sentinel.txt")).unwrap(),
        "untouched"
    );
    assert!(!outside.join("plugin.json").exists());
    assert!(!outside.join(".estate-pack-install.json").exists());
    assert_locked_cksum();
}

#[test]
fn plugin_install_refuses_thin_runner_docs_without_copying() {
    assert_locked_cksum();
    let home = scratch("thin");
    let out = home.join("export");
    let target = home.join("plugins-local");
    let estate = fixture().display().to_string();
    let out_s = out.display().to_string();
    let target_s = target.display().to_string();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "export-plugin",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    std::fs::write(out.join("RUNNER.md"), "# RUNNER\n\nthin\n").unwrap();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "plugin-install-local",
            "--check-only",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
            "--target",
            &target_s,
        ],
        &home,
        &[],
    );
    let combined = format!("{stdout}{stderr}");
    assert!(!ok, "{combined}");
    assert!(
        combined.contains("refuse:plugin-prove-runner-docs")
            || combined.contains("runner_docs: FAIL"),
        "{combined}"
    );
    assert!(combined.contains("refuse:plugin-install"), "{combined}");
    assert!(!target.join("research-crew").exists());
    assert_locked_cksum();
}

#[test]
fn plugin_install_refuses_bare_estate_bin_without_copying() {
    assert_locked_cksum();
    let home = scratch("bare");
    let out = home.join("export");
    let target = home.join("plugins-local");
    let estate = fixture().display().to_string();
    let out_s = out.display().to_string();
    let target_s = target.display().to_string();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "export-plugin",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
        ],
        &home,
        &[],
    );
    assert!(ok, "stderr={stderr}\nstdout={stdout}");
    let mcp_path = out.join("mcp.json");
    let mut mcp: Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
    let servers = mcp["mcpServers"].as_object_mut().unwrap();
    for server in servers.values_mut() {
        server["command"] = Value::String("estate".into());
    }
    std::fs::write(&mcp_path, format!("{mcp}\n")).unwrap();
    let (ok, stdout, stderr) = run(
        &[
            "pack",
            "plugin-install-local",
            "--check-only",
            "--id",
            "research-crew",
            "--estate",
            &estate,
            "--out",
            &out_s,
            "--target",
            &target_s,
        ],
        &home,
        &[],
    );
    let combined = format!("{stdout}{stderr}");
    assert!(!ok, "{combined}");
    assert!(
        combined.contains("refuse:plugin-prove-estate-bin"),
        "{combined}"
    );
    assert!(combined.contains("refuse:plugin-install"), "{combined}");
    assert!(!target.join("research-crew").exists());
    assert_locked_cksum();
}
