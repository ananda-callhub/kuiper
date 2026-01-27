use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::config::kuiper_data_dir;

const CACHE_FILE: &str = "cache.json";
const DEFAULT_TTL_HOURS: u64 = 24;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub response: String,
    pub model: String,
    pub timestamp: u64,
    pub ttl_hours: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ResponseCache {
    entries: HashMap<String, CacheEntry>,
}

impl ResponseCache {
    /// Load cache from disk
    pub fn load() -> Result<Self> {
        let path = cache_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)?;
        let cache: ResponseCache = serde_json::from_str(&content)?;

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let path = cache_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Get a cached response if valid
    pub fn get(&self, prompt: &str, model: &str) -> Option<&str> {
        let key = cache_key(prompt, model);

        if let Some(entry) = self.entries.get(&key) {
            if !is_expired(entry) {
                return Some(&entry.response);
            }
        }

        None
    }

    /// Store a response in the cache
    pub fn set(&mut self, prompt: &str, model: &str, response: &str) {
        let key = cache_key(prompt, model);

        let entry = CacheEntry {
            response: response.to_string(),
            model: model.to_string(),
            timestamp: current_timestamp(),
            ttl_hours: DEFAULT_TTL_HOURS,
        };

        self.entries.insert(key, entry);
    }

    /// Store a response with custom TTL
    pub fn set_with_ttl(&mut self, prompt: &str, model: &str, response: &str, ttl_hours: u64) {
        let key = cache_key(prompt, model);

        let entry = CacheEntry {
            response: response.to_string(),
            model: model.to_string(),
            timestamp: current_timestamp(),
            ttl_hours,
        };

        self.entries.insert(key, entry);
    }

    /// Remove expired entries
    pub fn cleanup(&mut self) -> usize {
        let before = self.entries.len();

        self.entries.retain(|_, entry| !is_expired(entry));

        before - self.entries.len()
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total = self.entries.len();
        let expired = self.entries.values().filter(|e| is_expired(e)).count();

        CacheStats {
            total_entries: total,
            valid_entries: total - expired,
            expired_entries: expired,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Get or compute a cached response
pub async fn get_or_compute<F, Fut>(
    prompt: &str,
    model: &str,
    compute: F,
) -> Result<String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let mut cache = ResponseCache::load().unwrap_or_default();

    // Check cache first
    if let Some(cached) = cache.get(prompt, model) {
        eprintln!("[Cache] Hit for {} model", model);
        return Ok(cached.to_string());
    }

    eprintln!("[Cache] Miss for {} model, computing...", model);

    // Compute the response
    let response = compute().await?;

    // Store in cache
    cache.set(prompt, model, &response);
    let _ = cache.save();

    Ok(response)
}

// Helper functions

fn cache_path() -> Result<PathBuf> {
    kuiper_data_dir()
        .map(|d| d.join(CACHE_FILE))
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))
}

fn cache_key(prompt: &str, model: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    model.hash(&mut hasher);

    format!("{}_{:x}", model, hasher.finish())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn is_expired(entry: &CacheEntry) -> bool {
    let now = current_timestamp();
    let age_hours = (now - entry.timestamp) / 3600;
    age_hours > entry.ttl_hours
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = cache_key("hello", "gemini");
        let key2 = cache_key("hello", "gemini");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_different() {
        let key1 = cache_key("hello", "gemini");
        let key2 = cache_key("hello", "claude");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_operations() {
        let mut cache = ResponseCache::default();

        assert!(cache.get("test", "gemini").is_none());

        cache.set("test", "gemini", "response");

        assert_eq!(cache.get("test", "gemini"), Some("response"));
        assert!(cache.get("test", "claude").is_none());
    }
}
