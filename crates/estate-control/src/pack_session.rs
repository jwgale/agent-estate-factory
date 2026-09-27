//! Pack-scoped short session for multi-hop complete memory.
//!
//! A crew/group holds a thread: successive `estate complete --pack`
//! (or pack MCP complete) calls can carry prior turns under one
//! session id. Transcripts live under `{state-dir}/pack-sessions/`
//! (throwaway-safe, not estate SoT). No secrets. Not a Grok Bot
//! server sync. Not a live PASS.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const SESSION_SCHEMA: &str = "cell-one.pack-session.v0";
pub(crate) const SESSION_DIR: &str = "pack-sessions";
pub(crate) const SESSION_ENV: &str = "CELL_PACK_SESSION";
pub(crate) const MAX_TURNS_ENV: &str = "CELL_PACK_SESSION_MAX_TURNS";
pub(crate) const MAX_BYTES_ENV: &str = "CELL_PACK_SESSION_MAX_BYTES";
pub(crate) const TTL_ENV: &str = "CELL_PACK_SESSION_TTL_SECS";

pub(crate) const DEFAULT_MAX_TURNS: u64 = 8;
pub(crate) const DEFAULT_MAX_BYTES: usize = 16 * 1024;
pub(crate) const DEFAULT_TTL_SECS: i64 = 24 * 60 * 60;
const RESULT_CAP: usize = 512;
const ID_MAX: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PackSession {
    pub schema: String,
    pub session_id: String,
    pub pack_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub ended: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub turns: Vec<SessionTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SessionTurn {
    pub seq: u64,
    pub prompt: String,
    pub result: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionStamp {
    pub session_id: String,
    pub session_turns: u64,
    pub session_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionRequest {
    None,
    ResumeOrCreate(String),
    CreateNew,
    CreateNamed(String),
}

#[derive(Debug, Clone)]
pub(crate) struct OpenSession {
    pub session: PackSession,
    pub context_applied: bool,
    pub created: bool,
    pub path: PathBuf,
    pub bounds: SessionBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionBounds {
    pub max_turns: u64,
    pub max_bytes: usize,
}

impl SessionRequest {
    pub(crate) fn from_flags(session: Option<&str>, session_create: bool) -> Result<Self> {
        let from_flag = session
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let from_env = std::env::var(SESSION_ENV)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let id = from_flag.or(from_env);
        match (id, session_create) {
            (None, false) => Ok(Self::None),
            (None, true) => Ok(Self::CreateNew),
            (Some(id), false) => Ok(Self::ResumeOrCreate(check_id("session", &id)?)),
            (Some(id), true) => Ok(Self::CreateNamed(check_id("session", &id)?)),
        }
    }

    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn session_root(state_dir: &Path) -> PathBuf {
    state_dir.join(SESSION_DIR)
}

pub(crate) fn session_path(state_dir: &Path, pack_id: &str, session_id: &str) -> PathBuf {
    session_root(state_dir)
        .join(estate_schema::normalize_name(pack_id))
        .join(format!("{session_id}.json"))
}

pub(crate) fn check_id(label: &str, raw: &str) -> Result<String> {
    let s = raw.trim();
    if s.is_empty() || s.len() > ID_MAX {
        bail!("refuse:session-id: {label} must be 1-{ID_MAX} chars");
    }
    if s.contains("..") || s.contains('/') || s.contains('\\') {
        bail!("refuse:session-id: {label} must not contain a path");
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("refuse:session-id: {label} must be [A-Za-z0-9_-]");
    }
    Ok(s.to_string())
}

pub(crate) fn mint_session_id(now: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(now.to_le_bytes());
    hasher.update(b"pack-session");
    hasher.update(std::process::id().to_le_bytes());
    if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
        hasher.update(dur.subsec_nanos().to_le_bytes());
    }
    let digest = hasher.finalize();
    format!("sess-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", digest[0], digest[1], digest[2], digest[3], digest[4], digest[5])
}

pub(crate) fn load_bounds() -> Result<SessionBounds> {
    Ok(SessionBounds {
        max_turns: parse_u64_env(MAX_TURNS_ENV, DEFAULT_MAX_TURNS)?,
        max_bytes: parse_usize_env(MAX_BYTES_ENV, DEFAULT_MAX_BYTES)?,
    })
}

pub(crate) fn load_ttl_secs(override_secs: Option<i64>) -> Result<Option<i64>> {
    if let Some(secs) = override_secs {
        return Ok(ttl_from_secs(secs));
    }
    match std::env::var(TTL_ENV) {
        Ok(raw) if !raw.trim().is_empty() => {
            let secs: i64 = raw.trim().parse().map_err(|_| {
                anyhow::anyhow!("refuse:session-ttl: {TTL_ENV} must be an integer (>= 0)")
            })?;
            if secs < 0 {
                bail!("refuse:session-ttl: {TTL_ENV} must be >= 0");
            }
            Ok(ttl_from_secs(secs))
        }
        _ => Ok(ttl_from_secs(DEFAULT_TTL_SECS)),
    }
}

fn ttl_from_secs(secs: i64) -> Option<i64> {
    if secs == 0 {
        None
    } else {
        Some(secs)
    }
}

fn parse_u64_env(key: &str, default: u64) -> Result<u64> {
    match std::env::var(key) {
        Ok(raw) if !raw.trim().is_empty() => {
            let n: u64 = raw.trim().parse().map_err(|_| {
                anyhow::anyhow!("refuse:session-bound: {key} must be a positive integer")
            })?;
            if n == 0 {
                bail!("refuse:session-bound: {key} must be >= 1");
            }
            Ok(n)
        }
        _ => Ok(default),
    }
}

fn parse_usize_env(key: &str, default: usize) -> Result<usize> {
    match std::env::var(key) {
        Ok(raw) if !raw.trim().is_empty() => {
            let n: usize = raw.trim().parse().map_err(|_| {
                anyhow::anyhow!("refuse:session-bound: {key} must be a positive integer")
            })?;
            if n == 0 {
                bail!("refuse:session-bound: {key} must be >= 1");
            }
            Ok(n)
        }
        _ => Ok(default),
    }
}

pub(crate) fn new_session(pack_id: &str, session_id: &str, now: i64, ttl_secs: Option<i64>) -> Result<PackSession> {
    let pack_id = check_id("pack", pack_id)?;
    let session_id = check_id("session", session_id)?;
    Ok(PackSession {
        schema: SESSION_SCHEMA.into(),
        session_id,
        pack_id,
        created_at: now,
        updated_at: now,
        expires_at: ttl_secs.map(|ttl| now + ttl),
        ended: false,
        turns: Vec::new(),
    })
}

pub(crate) fn load_session_file(path: &Path) -> Result<PackSession> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let session: PackSession = serde_json::from_str(&text)
        .with_context(|| format!("refuse:session: parse {}", path.display()))?;
    if session.schema != SESSION_SCHEMA {
        bail!(
            "refuse:session: schema must be {SESSION_SCHEMA} (got {})",
            session.schema
        );
    }
    check_id("session", &session.session_id)?;
    check_id("pack", &session.pack_id)?;
    Ok(session)
}

pub(crate) fn save_session(path: &Path, session: &PackSession) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(session)?;
    fs::write(path, format!("{body}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub(crate) fn find_session(state_dir: &Path, session_id: &str) -> Result<(PathBuf, PackSession)> {
    let id = check_id("session", session_id)?;
    let root = session_root(state_dir);
    if !root.is_dir() {
        bail!("refuse:session-missing: pack session '{id}' not under {}", root.display());
    }
    let mut hits = Vec::new();
    for pack_dir in fs::read_dir(&root).with_context(|| format!("read {}", root.display()))? {
        let pack_dir = pack_dir?;
        if !pack_dir.file_type()?.is_dir() {
            continue;
        }
        let path = pack_dir.path().join(format!("{id}.json"));
        if path.is_file() {
            hits.push(path);
        }
    }
    match hits.len() {
        0 => bail!("refuse:session-missing: pack session '{id}' not under {}", root.display()),
        1 => {
            let path = hits.remove(0);
            let session = load_session_file(&path)?;
            if session.session_id != id {
                bail!(
                    "refuse:session: file id '{}' does not match '{id}'",
                    session.session_id
                );
            }
            Ok((path, session))
        }
        _ => bail!("refuse:session-ambiguous: pack session '{id}' matches more than one pack"),
    }
}

pub(crate) fn load_for_pack(
    state_dir: &Path,
    pack_id: &str,
    session_id: &str,
) -> Result<(PathBuf, PackSession)> {
    let pack = check_id("pack", pack_id)?;
    let id = check_id("session", session_id)?;
    let path = session_path(state_dir, &pack, &id);
    if !path.is_file() {
        bail!(
            "refuse:session-missing: pack session '{id}' not under {}",
            path.parent().unwrap_or(state_dir).display()
        );
    }
    let session = load_session_file(&path)?;
    if estate_schema::normalize_name(&session.pack_id) != estate_schema::normalize_name(&pack) {
        bail!(
            "refuse:session-pack-mismatch: session pack is '{}', not '{}'",
            session.pack_id,
            pack
        );
    }
    Ok((path, session))
}

pub(crate) fn check_resumable(session: &PackSession, now: i64) -> Result<()> {
    if session.ended {
        bail!(
            "refuse:session-ended: pack session {} is ended",
            session.session_id
        );
    }
    if let Some(exp) = session.expires_at {
        if now > exp {
            bail!(
                "refuse:session-expired: pack session {} expired at {exp}",
                session.session_id
            );
        }
    }
    Ok(())
}

pub(crate) fn transcript_bytes(session: &PackSession) -> usize {
    session
        .turns
        .iter()
        .map(|t| t.prompt.len() + t.result.len())
        .sum()
}

pub(crate) fn check_bound(session: &PackSession, prompt: &str, bounds: SessionBounds) -> Result<()> {
    let turns = session.turns.len() as u64;
    if turns >= bounds.max_turns {
        bail!(
            "refuse:session-bound: pack session {} has {turns}/{} turns (max {})",
            session.session_id,
            bounds.max_turns,
            bounds.max_turns
        );
    }
    let used = transcript_bytes(session);
    let next = used.saturating_add(prompt.len());
    if next > bounds.max_bytes {
        bail!(
            "refuse:session-bound: pack session {} would exceed {} bytes (used {used}, prompt {})",
            session.session_id,
            bounds.max_bytes,
            prompt.len()
        );
    }
    Ok(())
}

pub(crate) fn redact_secrets(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_inclusive([' ', '\n', '\t', ',', ';']) {
        let core = word.trim_end_matches([' ', '\n', '\t', ',', ';']);
        if looks_secret(core) {
            let sep = &word[core.len()..];
            out.push_str("[redacted]");
            out.push_str(sep);
        } else {
            out.push_str(word);
        }
    }
    out
}

fn looks_secret(token: &str) -> bool {
    let t = token.trim();
    (t.starts_with("sk-") || t.starts_with("xai-") || t.starts_with("xai_")) && t.len() >= 12
}

pub(crate) fn compact_result(text: &str) -> String {
    let redacted = redact_secrets(text.trim());
    truncate(&redacted, RESULT_CAP)
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

pub(crate) fn render_context(session: &PackSession) -> String {
    if session.turns.is_empty() {
        return String::new();
    }
    let mut out = format!(
        "[pack session {} pack={} turns={}]\n",
        session.session_id,
        session.pack_id,
        session.turns.len()
    );
    for turn in &session.turns {
        out.push_str(&format!(
            "{}. user: {}\n   assistant: {}\n",
            turn.seq, turn.prompt, turn.result
        ));
    }
    out.push('\n');
    out
}

pub(crate) fn prepend_context(session: &PackSession, prompt: &str) -> String {
    let prefix = render_context(session);
    if prefix.is_empty() {
        prompt.to_string()
    } else {
        format!("{prefix}{prompt}")
    }
}

impl OpenSession {
    pub(crate) fn stamp(&self) -> SessionStamp {
        SessionStamp {
            session_id: self.session.session_id.clone(),
            session_turns: self.session.turns.len() as u64,
            session_context: self.context_applied,
        }
    }

    pub(crate) fn prepend(&self, prompt: &str) -> String {
        prepend_context(&self.session, prompt)
    }

    pub(crate) fn check_can_append(&self, prompt: &str) -> Result<()> {
        check_resumable(&self.session, now_unix())?;
        check_bound(&self.session, prompt, self.bounds)
    }

    pub(crate) fn append_turn(
        &mut self,
        prompt: &str,
        result: &str,
        agent: &str,
        receipt_id: Option<&str>,
    ) -> Result<SessionStamp> {
        self.check_can_append(prompt)?;
        let now = now_unix();
        let seq = self.session.turns.len() as u64 + 1;
        self.session.turns.push(SessionTurn {
            seq,
            prompt: redact_secrets(prompt),
            result: compact_result(result),
            agent: agent.to_string(),
            receipt_id: receipt_id.map(|s| s.to_string()),
        });
        self.session.updated_at = now;
        save_session(&self.path, &self.session)?;
        Ok(SessionStamp {
            session_id: self.session.session_id.clone(),
            session_turns: self.session.turns.len() as u64,
            session_context: self.context_applied,
        })
    }
}

/// True when a bound session can still take `hops` more turns.
/// Ended / expired / missing / bound → false (tick then mints a fresh id).
/// Explicit `--session` still refuses; this is only the auto-bind probe.
pub(crate) fn session_is_reusable(
    state_dir: &Path,
    pack_id: &str,
    session_id: &str,
    hops: usize,
) -> bool {
    let Ok((_, session)) = load_for_pack(state_dir, pack_id, session_id) else {
        return false;
    };
    if check_resumable(&session, now_unix()).is_err() {
        return false;
    }
    let Ok(bounds) = load_bounds() else {
        return false;
    };
    let remaining = bounds.max_turns.saturating_sub(session.turns.len() as u64);
    remaining >= hops.max(1) as u64 && transcript_bytes(&session) < bounds.max_bytes
}

/// Reuse `bound_id` when still open; otherwise mint a new pack session.
/// Does not change ended/expired/bound refuse on an explicit `--session`.
pub(crate) fn bind_or_create_session(
    state_dir: &Path,
    pack_id: &str,
    bound_id: Option<&str>,
    hops: usize,
) -> Result<(String, bool)> {
    if let Some(id) = bound_id {
        if session_is_reusable(state_dir, pack_id, id, hops) {
            return Ok((id.to_string(), false));
        }
    }
    let opened = open_for_complete(state_dir, pack_id, &SessionRequest::CreateNew)?
        .ok_or_else(|| anyhow::anyhow!("refuse:session: could not open pack session"))?;
    Ok((opened.session.session_id, true))
}

pub(crate) fn open_for_complete(
    state_dir: &Path,
    pack_id: &str,
    request: &SessionRequest,
) -> Result<Option<OpenSession>> {
    match request {
        SessionRequest::None => Ok(None),
        SessionRequest::CreateNew => {
            let id = mint_session_id(now_unix());
            Ok(Some(create_open(state_dir, pack_id, &id, None)?))
        }
        SessionRequest::CreateNamed(id) => {
            let path = session_path(state_dir, pack_id, id);
            if path.is_file() {
                bail!("refuse:session-exists: pack session '{id}' already exists");
            }
            Ok(Some(create_open(state_dir, pack_id, id, None)?))
        }
        SessionRequest::ResumeOrCreate(id) => {
            let path = session_path(state_dir, pack_id, id);
            if path.is_file() {
                Ok(Some(resume_open(state_dir, pack_id, id)?))
            } else {
                Ok(Some(create_open(state_dir, pack_id, id, None)?))
            }
        }
    }
}

fn create_open(
    state_dir: &Path,
    pack_id: &str,
    session_id: &str,
    ttl_override: Option<i64>,
) -> Result<OpenSession> {
    let now = now_unix();
    let ttl = load_ttl_secs(ttl_override)?;
    let session = new_session(pack_id, session_id, now, ttl)?;
    let path = session_path(state_dir, &session.pack_id, &session.session_id);
    if path.is_file() {
        bail!(
            "refuse:session-exists: pack session '{}' already exists",
            session.session_id
        );
    }
    save_session(&path, &session)?;
    Ok(OpenSession {
        session,
        context_applied: false,
        created: true,
        path,
        bounds: load_bounds()?,
    })
}

fn resume_open(state_dir: &Path, pack_id: &str, session_id: &str) -> Result<OpenSession> {
    let (path, session) = load_for_pack(state_dir, pack_id, session_id)?;
    check_resumable(&session, now_unix())?;
    let context_applied = !session.turns.is_empty();
    Ok(OpenSession {
        session,
        context_applied,
        created: false,
        path,
        bounds: load_bounds()?,
    })
}

pub(crate) fn status_line(stamp: &SessionStamp) -> String {
    format!(
        "pack session: id={} turns={} context={}",
        stamp.session_id,
        stamp.session_turns,
        if stamp.session_context {
            "applied"
        } else {
            "none"
        }
    )
}

pub(crate) fn cmd_pack_session_create(
    pack: &str,
    estate_path: &Path,
    state_dir: &Path,
    ttl_secs: Option<i64>,
    session_id: Option<&str>,
) -> Result<()> {
    let estate = estate_schema::load_estate(estate_path)
        .with_context(|| format!("load {}", estate_path.display()))?;
    let row = estate.pack(pack).ok_or_else(|| {
        anyhow::anyhow!("refuse:unknown-pack: pack '{pack}' not on estate")
    })?;
    let id = match session_id {
        Some(raw) => check_id("session", raw)?,
        None => mint_session_id(now_unix()),
    };
    let opened = create_open(state_dir, &row.id, &id, ttl_secs)?;
    println!(
        "pack session: created id={} pack={} turns=0 expires_at={}",
        opened.session.session_id,
        opened.session.pack_id,
        opened
            .session
            .expires_at
            .map(|e| e.to_string())
            .unwrap_or_else(|| "-".into())
    );
    Ok(())
}

pub(crate) fn cmd_pack_session_show(
    session_id: &str,
    pack: Option<&str>,
    state_dir: &Path,
) -> Result<()> {
    let (_path, session) = match pack {
        Some(pack_id) => load_for_pack(state_dir, pack_id, session_id)?,
        None => find_session(state_dir, session_id)?,
    };
    let now = now_unix();
    let expired = session
        .expires_at
        .map(|exp| now > exp)
        .unwrap_or(false);
    println!(
        "pack session: {} pack={} turns={} ended={} expired={}",
        session.session_id,
        session.pack_id,
        session.turns.len(),
        session.ended,
        expired
    );
    if let Some(exp) = session.expires_at {
        println!("  expires_at={exp}");
    }
    if session.turns.is_empty() {
        println!("  (no turns)");
        return Ok(());
    }
    for turn in &session.turns {
        println!("  {}. user: {}", turn.seq, turn.prompt);
        println!("     assistant: {}", turn.result);
    }
    Ok(())
}

pub(crate) fn cmd_pack_session_end(
    session_id: &str,
    pack: Option<&str>,
    state_dir: &Path,
) -> Result<()> {
    let (path, mut session) = match pack {
        Some(pack_id) => load_for_pack(state_dir, pack_id, session_id)?,
        None => find_session(state_dir, session_id)?,
    };
    if session.ended {
        println!(
            "pack session: already ended id={} pack={}",
            session.session_id, session.pack_id
        );
        return Ok(());
    }
    session.ended = true;
    session.updated_at = now_unix();
    save_session(&path, &session)?;
    println!(
        "pack session: ended id={} pack={} turns={}",
        session.session_id,
        session.pack_id,
        session.turns.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cell-pack-session-unit-{name}-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn mint_id_is_filesystem_safe() {
        let id = mint_session_id(1);
        assert!(id.starts_with("sess-"), "{id}");
        assert!(check_id("session", &id).is_ok(), "{id}");
    }

    #[test]
    fn redact_strips_key_shaped_tokens() {
        let out = redact_secrets("hello xai-not-a-real-key-at-all world");
        assert!(out.contains("[redacted]"), "{out}");
        assert!(!out.contains("xai-not-a-real-key-at-all"), "{out}");
    }

    #[test]
    fn bound_refuses_max_turns_and_bytes() {
        let mut session = new_session("research-crew", "sess-aabbccddeeff", 10, Some(60)).unwrap();
        session.turns.push(SessionTurn {
            seq: 1,
            prompt: "ping".into(),
            result: "pong".into(),
            agent: "research".into(),
            receipt_id: None,
        });
        let err = check_bound(
            &session,
            "next",
            SessionBounds {
                max_turns: 1,
                max_bytes: 16 * 1024,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("refuse:session-bound"), "{err}");
        assert!(err.contains("1/1"), "{err}");

        session.turns.clear();
        let err = check_bound(
            &session,
            "abcdefghij",
            SessionBounds {
                max_turns: 8,
                max_bytes: 4,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("refuse:session-bound"), "{err}");
        assert!(err.contains("bytes"), "{err}");
    }

    #[test]
    fn ended_and_expired_refuse_resume() {
        let mut session = new_session("research-crew", "sess-aabbccddeeff", 10, Some(5)).unwrap();
        session.ended = true;
        let err = check_resumable(&session, 11).unwrap_err().to_string();
        assert!(err.contains("refuse:session-ended"), "{err}");
        session.ended = false;
        let err = check_resumable(&session, 16).unwrap_err().to_string();
        assert!(err.contains("refuse:session-expired"), "{err}");
        check_resumable(&session, 14).unwrap();
    }

    #[test]
    fn context_includes_prior_turns_only() {
        let mut session = new_session("research-crew", "sess-aabbccddeeff", 10, None).unwrap();
        assert!(render_context(&session).is_empty());
        session.turns.push(SessionTurn {
            seq: 1,
            prompt: "unique-hop-alpha".into(),
            result: "mock:unique-hop-alpha".into(),
            agent: "research".into(),
            receipt_id: None,
        });
        let text = prepend_context(&session, "follow-up");
        assert!(text.contains("unique-hop-alpha"), "{text}");
        assert!(text.contains("follow-up"), "{text}");
        assert!(text.contains("[pack session sess-aabbccddeeff"), "{text}");
    }

    #[test]
    fn create_append_and_find_roundtrip() {
        let dir = scratch("roundtrip");
        let mut opened = create_open(&dir, "research-crew", "sess-roundtrip1", Some(60)).unwrap();
        assert!(opened.created);
        assert!(!opened.context_applied);
        opened
            .append_turn("ping", "mock:ping", "research", Some("r-1"))
            .unwrap();
        let resumed = resume_open(&dir, "research-crew", "sess-roundtrip1").unwrap();
        assert!(resumed.context_applied);
        assert_eq!(resumed.session.turns.len(), 1);
        let (_path, found) = find_session(&dir, "sess-roundtrip1").unwrap();
        assert_eq!(found.pack_id, "research-crew");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_chars_refuse() {
        assert!(check_id("session", "../escape").is_err());
        assert!(check_id("session", "sess/abc").is_err());
        assert!(check_id("session", "sess abc").is_err());
    }

    #[test]
    fn bind_reuses_open_session_and_mints_when_ended() {
        let dir = scratch("bind-reuse");
        let (first, created) =
            bind_or_create_session(&dir, "research-crew", None, 2).unwrap();
        assert!(created);
        assert!(first.starts_with("sess-"), "{first}");
        let (again, created) =
            bind_or_create_session(&dir, "research-crew", Some(&first), 2).unwrap();
        assert!(!created);
        assert_eq!(again, first);
        assert!(session_is_reusable(&dir, "research-crew", &first, 2));

        cmd_pack_session_end(&first, Some("research-crew"), &dir).unwrap();
        assert!(!session_is_reusable(&dir, "research-crew", &first, 2));
        let (fresh, created) =
            bind_or_create_session(&dir, "research-crew", Some(&first), 2).unwrap();
        assert!(created);
        assert_ne!(fresh, first);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bind_mints_when_remaining_turns_cannot_cover_hops() {
        let dir = scratch("bind-bound");
        let mut opened = create_open(&dir, "research-crew", "sess-boundturns01", None).unwrap();
        opened
            .append_turn("ping", "mock:ping", "research", None)
            .unwrap();
        opened
            .append_turn("pong", "mock:pong", "horizon", None)
            .unwrap();
        // Default max turns is 8: two used, two more hops still fit.
        assert!(session_is_reusable(&dir, "research-crew", "sess-boundturns01", 2));
        // Asking for more hops than remaining turns treats the session as bound.
        assert!(!session_is_reusable(&dir, "research-crew", "sess-boundturns01", 8));
        let (fresh, created) =
            bind_or_create_session(&dir, "research-crew", Some("sess-boundturns01"), 8).unwrap();
        assert!(created);
        assert_ne!(fresh, "sess-boundturns01");
        let _ = fs::remove_dir_all(&dir);
    }
}
