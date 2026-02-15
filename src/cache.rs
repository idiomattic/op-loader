use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRemoval {
    Removed,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheKind {
    EnvInject,
    TemplateRender,
}

pub fn lock_path_for_account(
    cache_root: &std::path::Path,
    account_id: &str,
    kind: CacheKind,
) -> PathBuf {
    let prefix = match kind {
        CacheKind::EnvInject => "op_inject_env",
        CacheKind::TemplateRender => "op_inject_template",
    };
    let filename = format!("{}_{}.lock", prefix, sanitize_account_id(account_id));
    cache_root.join(filename)
}

pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(dir).join("op_loader"));
    }

    let home = std::env::var_os("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".cache").join("op_loader"))
}

pub fn ensure_cache_dir() -> Result<PathBuf> {
    let dir = cache_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cache directory: {}", dir.display()))?;
    Ok(dir)
}

pub fn cache_path_for_account(
    cache_root: &std::path::Path,
    account_id: &str,
    kind: CacheKind,
) -> PathBuf {
    let prefix = match kind {
        CacheKind::EnvInject => "op_inject_env",
        CacheKind::TemplateRender => "op_inject_template",
    };
    let filename = format!("{}_{}.cache", prefix, sanitize_account_id(account_id));
    cache_root.join(filename)
}

pub fn cache_file_for_account(account_id: &str, kind: CacheKind) -> Result<PathBuf> {
    Ok(cache_path_for_account(&cache_dir()?, account_id, kind))
}

pub fn cache_lock_path_for_account(account_id: &str, kind: CacheKind) -> Result<PathBuf> {
    Ok(lock_path_for_account(&cache_dir()?, account_id, kind))
}

pub fn remove_cache_for_account(account_id: &str) -> Result<CacheRemoval> {
    let mut removed_any = false;
    for kind in [CacheKind::EnvInject, CacheKind::TemplateRender] {
        let path = cache_file_for_account(account_id, kind)?;
        if !path.exists() {
            continue;
        }

        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove cache file: {}", path.display()))?;
        removed_any = true;
    }

    if removed_any {
        Ok(CacheRemoval::Removed)
    } else {
        Ok(CacheRemoval::NotFound)
    }
}

fn sanitize_account_id(account_id: &str) -> String {
    let mut sanitized = String::with_capacity(account_id.len());
    for ch in account_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "account".to_string()
    } else {
        sanitized
    }
}
