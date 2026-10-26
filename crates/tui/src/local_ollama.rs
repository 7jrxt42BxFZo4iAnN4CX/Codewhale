//! Live local Ollama catalog facts and unconfigured first-run adoption.
//!
//! Virgin sessions default to the DeepSeek costume (`deepseek-flash`). When a
//! real local daemon answers `GET /api/tags` (or the OpenAI-compat
//! `GET /v1/models` roster), the painted route must switch to a tag that
//! actually exists. Discovery never replaces an explicit provider/model,
//! including a configured route whose credentials need repair.

use std::time::Duration;

use codewhale_config::catalog::{
    CatalogOffering, CatalogSource, ProviderCatalogDelta, base_url_fingerprint, now_unix,
};
use serde::Deserialize;

use crate::config::Config;
use codewhale_config::ProviderKind;

const TAGS_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Upper bound on `/api/show` lookups per probe. A developer box can hold
/// dozens of tags; ranking needs only the plausible chat candidates.
const SHOW_PROBE_LIMIT: usize = 8;

/// Result of a successful local Ollama tags/models probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveLocalOllamaCatalog {
    pub(crate) endpoint_v1: String,
    pub(crate) tags: Vec<String>,
    /// The tag adoption may switch to: a model that can hold a conversation.
    /// `None` when every live tag is an embedding/reranker model — adopting
    /// one of those would make every first message fail.
    pub(crate) chat_tag: Option<String>,
}

impl LiveLocalOllamaCatalog {
    /// The chat-capable tag to adopt, if the catalog has one.
    pub(crate) fn preferred_tag(&self) -> Option<&str> {
        self.chat_tag.as_deref()
    }
}

/// What `/api/show` reports about one tag. Both fields are optional because
/// older daemons omit `capabilities` and some architectures omit a context
/// length; a missing fact is unknown, never a "no".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OllamaTagProfile {
    pub(crate) capabilities: Option<Vec<String>>,
    pub(crate) context_length: Option<u64>,
}

impl OllamaTagProfile {
    fn has_capability(&self, name: &str) -> Option<bool> {
        self.capabilities
            .as_ref()
            .map(|caps| caps.iter().any(|cap| cap.eq_ignore_ascii_case(name)))
    }
}

/// Name heuristic for tags that cannot chat: embedding and reranking models.
/// Used only when the daemon did not report capabilities.
pub(crate) fn looks_like_non_chat_tag(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    ["embed", "bge", "rerank", "minilm"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn looks_like_coder_tag(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    lower.contains("coder") || lower.contains("code")
}

/// Pick the tag adoption should switch to.
///
/// A tag is a chat candidate when `/api/show` lists `completion`, or — when
/// the daemon reported no capabilities — when its name is not an embedding or
/// reranker. Among candidates: coder or tool-capable models first, then the
/// largest reported context, then alphabetical order for stability.
pub(crate) fn choose_chat_tag(
    tags: &[String],
    profiles: &std::collections::HashMap<String, OllamaTagProfile>,
) -> Option<String> {
    let unknown = OllamaTagProfile::default();
    tags.iter()
        .filter_map(|tag| {
            let profile = profiles.get(tag).unwrap_or(&unknown);
            let chat = match profile.has_capability("completion") {
                Some(known) => known,
                None => !looks_like_non_chat_tag(tag),
            };
            if !chat {
                return None;
            }
            let preferred =
                looks_like_coder_tag(tag) || profile.has_capability("tools").unwrap_or(false);
            Some((
                preferred,
                profile.context_length.unwrap_or(0),
                std::cmp::Reverse(tag.as_str()),
                tag,
            ))
        })
        .max_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)))
        .map(|(_, _, _, tag)| tag.clone())
}

#[derive(Debug, Deserialize)]
struct OllamaShowResponse {
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    model_info: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Parse `POST /api/show` JSON into the facts adoption ranks on.
pub(crate) fn parse_ollama_show_response(payload: &str) -> anyhow::Result<OllamaTagProfile> {
    let parsed: OllamaShowResponse = serde_json::from_str(payload)
        .map_err(|err| anyhow::anyhow!("Failed to parse Ollama /api/show JSON: {err}"))?;
    let context_length = parsed.model_info.as_ref().and_then(|info| {
        info.iter()
            .filter(|(key, _)| key.ends_with(".context_length"))
            .find_map(|(_, value)| value.as_u64())
    });
    Ok(OllamaTagProfile {
        capabilities: parsed.capabilities,
        context_length,
    })
}

/// True when this session should adopt a live local catalog into chrome.
///
/// An unconfigured first run paints DeepSeek by default; a live local
/// roster can replace that default. Explicit routes are left alone,
/// and so is a provider picker the person has already started using: the
/// probe answers late, and switching provider under them would close the
/// picker mid-choice or mid-key.
#[must_use]
pub(crate) fn should_adopt_live_local_ollama(
    startup_route_configured: bool,
    provider: ProviderKind,
    picker_interacted: bool,
    onboarding_needs_key: bool,
) -> bool {
    if startup_route_configured {
        return false;
    }
    if provider == ProviderKind::Ollama {
        // Already on Ollama — route_runtime + #5795 own the tag; don't fight it.
        return false;
    }
    if picker_interacted {
        return false;
    }
    onboarding_needs_key
}

/// Resolve the OpenAI-compat Ollama base URL (`…/v1`) from config defaults.
pub(crate) fn ollama_v1_base_url(config: &Config) -> String {
    config
        .resolve_provider_pin_identity("ollama")
        .ok()
        .filter(|identity| identity.provider == ProviderKind::Ollama)
        .map(|identity| config.base_url_for_route(&identity))
        .unwrap_or_default()
}

/// Strip a trailing `/v1` (with optional slash) so we can hit native `/api/tags`.
pub(crate) fn ollama_native_origin(v1_base: &str) -> String {
    let trimmed = v1_base.trim().trim_end_matches('/');
    if let Some(origin) = trimmed.strip_suffix("/v1") {
        origin.to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagModel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    details: Option<serde_json::Value>,
    #[serde(default)]
    context_length: Option<u64>,
}

/// Parse native Ollama `GET /api/tags` JSON into sorted unique tag ids.
#[cfg(test)]
pub(crate) fn parse_ollama_tags_response(payload: &str) -> anyhow::Result<Vec<String>> {
    parse_ollama_tags_with_profiles(payload).map(|(tags, _)| tags)
}

pub(crate) fn parse_ollama_tags_with_profiles(
    payload: &str,
) -> anyhow::Result<(
    Vec<String>,
    std::collections::HashMap<String, OllamaTagProfile>,
)> {
    let parsed: OllamaTagsResponse = serde_json::from_str(payload)
        .map_err(|err| anyhow::anyhow!("Failed to parse Ollama /api/tags JSON: {err}"))?;
    anyhow::ensure!(
        parsed.models.len() <= crate::client::PROVIDER_CATALOG_MAX_ROWS,
        "Ollama catalog exceeds the model row limit"
    );
    let mut profiles = std::collections::HashMap::new();
    for row in parsed.models {
        let tag = if row.name.trim().is_empty() {
            row.model.trim()
        } else {
            row.name.trim()
        };
        if !crate::provider_lake::valid_catalog_model_id(tag) {
            continue;
        }
        let profile = OllamaTagProfile {
            capabilities: row.capabilities,
            context_length: row
                .details
                .as_ref()
                .and_then(|details| details.get("context_length"))
                .and_then(serde_json::Value::as_u64),
        };
        if let Some(previous) = profiles.insert(tag.to_string(), profile.clone()) {
            anyhow::ensure!(
                previous == profile,
                "Ollama reported conflicting tag capabilities"
            );
        }
    }
    let mut tags: Vec<String> = profiles.keys().cloned().collect();
    tags.sort();
    Ok((tags, profiles))
}

pub(crate) fn ollama_catalog_delta(
    provider: &str,
    endpoint_v1: &str,
    tags: &[String],
    profiles: &std::collections::HashMap<String, OllamaTagProfile>,
    running_body: Option<&str>,
) -> ProviderCatalogDelta {
    let running: std::collections::HashMap<String, u64> = running_body
        .and_then(|body| serde_json::from_str::<OllamaTagsResponse>(body).ok())
        .filter(|response| response.models.len() <= crate::client::PROVIDER_CATALOG_MAX_ROWS)
        .into_iter()
        .flat_map(|response| response.models)
        .filter_map(|row| {
            let context = row.context_length.filter(|value| *value > 0)?;
            let context = u32::try_from(context).ok().map(u64::from)?;
            let tag = if row.name.trim().is_empty() {
                row.model.trim()
            } else {
                row.name.trim()
            };
            (!tag.is_empty()).then(|| (tag.to_string(), context))
        })
        .collect();
    let chat_tags: Vec<String> = tags
        .iter()
        .filter(|tag| crate::provider_lake::valid_catalog_model_id(tag))
        .filter(|tag| {
            profiles
                .get(*tag)
                .is_some_and(|profile| profile.has_capability("completion") == Some(true))
        })
        .cloned()
        .collect();
    let preferred = choose_chat_tag(&chat_tags, profiles);
    let fingerprint = base_url_fingerprint(endpoint_v1);
    let fetched_at = now_unix();
    let offerings = chat_tags
        .iter()
        .filter_map(|tag| {
            let profile = profiles.get(tag)?;
            Some(CatalogOffering {
                provider: provider.to_string(),
                wire_model_id: tag.clone(),
                endpoint_key: "chat".into(),
                default_for_provider: preferred.as_deref() == Some(tag.as_str()),
                tool_call: profile.has_capability("tools"),
                reasoning: profile.has_capability("thinking"),
                limit: running.get(tag).copied().map(|context| {
                    codewhale_config::models_dev::ModelsDevLimit {
                        context: Some(context),
                        ..Default::default()
                    }
                }),
                source: CatalogSource::Live {
                    base_url_fingerprint: fingerprint.clone(),
                    fetched_at,
                },
                ..Default::default()
            })
        })
        .collect();
    ProviderCatalogDelta {
        provider: provider.to_string(),
        base_url_fingerprint: fingerprint,
        fetched_at,
        offerings,
    }
}

fn record_ollama_tags_into_lake(
    endpoint_v1: &str,
    tags: &[String],
    profiles: &std::collections::HashMap<String, OllamaTagProfile>,
    running_body: Option<&str>,
) -> Option<String> {
    let delta = ollama_catalog_delta("ollama", endpoint_v1, tags, profiles, running_body);
    let chat_tag = delta
        .offerings
        .iter()
        .find(|offering| offering.default_for_provider)
        .map(|offering| offering.wire_model_id.clone());
    let ticket = crate::provider_catalog_live::begin_refresh_for_identity(
        ProviderKind::Ollama,
        "ollama",
        endpoint_v1,
    );
    if matches!(
        crate::provider_catalog_live::record_success_if_current(&ticket, delta),
        Some(codewhale_config::catalog::CatalogStatus::Fresh)
    ) {
        chat_tag
    } else {
        None
    }
}

fn probe_client() -> anyhow::Result<reqwest::Client> {
    // The first-run probe can run before any provider client has installed
    // the rustls crypto provider; the shared builder installs it (the bare
    // `reqwest::Client::builder()` panics under `rustls-no-provider`).
    Ok(codewhale_release::tls::reqwest_client_builder()
        .timeout(TAGS_PROBE_TIMEOUT)
        .build()?)
}

async fn fetch_text(url: &str) -> anyhow::Result<String> {
    let response = probe_client()?.get(url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }
    crate::client::bounded_provider_catalog_text(
        response,
        crate::client::PROVIDER_CATALOG_MAX_RESPONSE_BYTES,
    )
    .await
    .map_err(|reason| anyhow::anyhow!("Ollama catalog response rejected: {reason:?}"))
}

async fn fetch_tag_profile(origin: &str, tag: &str) -> anyhow::Result<OllamaTagProfile> {
    let response = probe_client()?
        .post(format!("{origin}/api/show"))
        .json(&serde_json::json!({ "model": tag }))
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }
    let body = crate::client::bounded_provider_catalog_text(
        response,
        crate::client::PROVIDER_CATALOG_MAX_RESPONSE_BYTES,
    )
    .await
    .map_err(|reason| anyhow::anyhow!("Ollama profile response rejected: {reason:?}"))?;
    parse_ollama_show_response(&body)
}

/// Ask `/api/show` about plausible tags whose capabilities are still unknown.
/// An unprofiled tag cannot become an executable catalog default.
pub(crate) async fn fetch_tag_profiles<F, Fut>(
    tags: &[String],
    profiles: &std::collections::HashMap<String, OllamaTagProfile>,
    mut fetch: F,
) -> std::collections::HashMap<String, OllamaTagProfile>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<OllamaTagProfile>>,
{
    let candidates: Vec<&String> = tags
        .iter()
        .filter(|tag| {
            profiles
                .get(*tag)
                .is_none_or(|profile| profile.capabilities.is_none())
                && !looks_like_non_chat_tag(tag)
        })
        .take(SHOW_PROBE_LIMIT)
        .collect();
    let results =
        futures_util::future::join_all(candidates.iter().map(|tag| fetch((*tag).clone()))).await;
    candidates
        .into_iter()
        .zip(results)
        .filter_map(|(tag, result)| match result {
            Ok(profile) => Some((tag.clone(), profile)),
            Err(err) => {
                tracing::debug!(target: "local_ollama", error = %err, tag = %tag, "POST /api/show probe failed");
                None
            }
        })
        .collect()
}

/// Probe local Ollama for a live catalog. Prefers native `/api/tags`, falls
/// back to OpenAI-compat `/v1/models`. Returns `None` when nothing useful
/// answered — never invents a tag.
pub(crate) async fn probe_live_local_ollama_catalog(
    config: &Config,
) -> Option<LiveLocalOllamaCatalog> {
    let endpoint_v1 = ollama_v1_base_url(config);
    let origin = ollama_native_origin(&endpoint_v1);
    let tags_url = format!("{origin}/api/tags");
    let (tags, mut profiles) = match fetch_text(&tags_url).await {
        Ok(body) => match parse_ollama_tags_with_profiles(&body) {
            Ok((tags, profiles)) if !tags.is_empty() => (tags, profiles),
            Ok(_) => return None,
            Err(err) => {
                tracing::debug!(target: "local_ollama", error = %err, "GET /api/tags returned unusable body");
                (Vec::new(), std::collections::HashMap::new())
            }
        },
        Err(err) => {
            tracing::debug!(target: "local_ollama", error = %err, url = %tags_url, "GET /api/tags probe failed");
            (Vec::new(), std::collections::HashMap::new())
        }
    };
    let tags = if tags.is_empty() {
        let models_url = format!("{}/models", endpoint_v1.trim_end_matches('/'));
        match fetch_text(&models_url).await {
            Ok(body) => match crate::client::parse_models_response(&body) {
                Ok(models) if !models.is_empty() => {
                    models.into_iter().map(|model| model.id).collect()
                }
                _ => return None,
            },
            Err(_) => return None,
        }
    } else {
        tags
    };
    let fetched = fetch_tag_profiles(&tags, &profiles, |tag| {
        let origin = &origin;
        async move { fetch_tag_profile(origin, &tag).await }
    })
    .await;
    for (tag, profile) in fetched {
        profiles
            .entry(tag)
            .and_modify(|known| {
                if known.capabilities.is_none() {
                    known.capabilities = profile.capabilities.clone();
                }
                if known.context_length.is_none() {
                    known.context_length = profile.context_length;
                }
            })
            .or_insert(profile);
    }
    let running_body = fetch_text(&format!("{origin}/api/ps")).await.ok();
    let chat_tag =
        record_ollama_tags_into_lake(&endpoint_v1, &tags, &profiles, running_body.as_deref());
    Some(LiveLocalOllamaCatalog {
        endpoint_v1,
        tags,
        chat_tag,
    })
}

/// Env opt-out for harnesses that must not see the developer's machine.
///
/// `spawn_local_ollama_adoption_probe` is already inert under `cfg(test)`, but
/// the PTY suites spawn the real binary, so that guard never reaches them. A
/// developer running Ollama on :11434 therefore gets the launch screen replaced
/// by a "Provider switched: deepseek -> ollama" notice, and the PTY tests that
/// wait for launch text fail on their machine while CI stays green. Sealing the
/// HOME is not enough, because this leak arrives over the loopback network
/// rather than through the filesystem.
pub(crate) const DISABLE_LOCAL_OLLAMA_PROBE_ENV: &str = "CODEWHALE_DISABLE_LOCAL_OLLAMA_PROBE";

fn local_ollama_probe_disabled() -> bool {
    std::env::var_os(DISABLE_LOCAL_OLLAMA_PROBE_ENV).is_some_and(|value| !value.is_empty())
}

/// Background probe used by the event loop (mirrors `spawn_startup_version_check`).
pub(crate) fn spawn_local_ollama_adoption_probe(
    config: &Config,
    should_probe: bool,
) -> Option<tokio::task::JoinHandle<Option<LiveLocalOllamaCatalog>>> {
    if !should_probe || local_ollama_probe_disabled() {
        return None;
    }
    #[cfg(test)]
    {
        let _ = config;
        None
    }
    #[cfg(not(test))]
    {
        let config = config.clone();
        Some(tokio::spawn(async move {
            probe_live_local_ollama_catalog(&config).await
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, lock_test_env};
    use std::collections::HashMap;

    #[test]
    fn parse_ollama_tags_response_reads_name_field() {
        let body = r#"{"models":[{"name":"qwen2.5:0.5b","model":"qwen2.5:0.5b","size":0}]}"#;
        let tags = parse_ollama_tags_response(body).expect("parse");
        assert_eq!(tags, vec!["qwen2.5:0.5b".to_string()]);
    }

    #[test]
    fn parse_ollama_tags_response_sorts_and_dedups() {
        let body = r#"{"models":[
            {"name":"zeta:tag"},
            {"name":"alpha:tag"},
            {"name":"alpha:tag"}
        ]}"#;
        let tags = parse_ollama_tags_response(body).expect("parse");
        assert_eq!(tags, vec!["alpha:tag".to_string(), "zeta:tag".to_string()]);
    }

    #[test]
    fn ollama_native_origin_strips_v1() {
        assert_eq!(
            ollama_native_origin("http://localhost:11434/v1"),
            "http://localhost:11434"
        );
        assert_eq!(
            ollama_native_origin("http://127.0.0.1:11434/v1/"),
            "http://127.0.0.1:11434"
        );
    }

    fn tags(names: &[&str]) -> Vec<String> {
        let mut tags: Vec<String> = names.iter().map(|name| (*name).to_string()).collect();
        tags.sort();
        tags
    }

    #[test]
    fn chat_tag_never_picks_an_embedding_model() {
        // The installed-0.10.0 re-run adopted `nomic-embed-text:latest`
        // because it sorted first; with no /api/show facts the name decides.
        let tags = tags(&["qwen2.5-coder:7b", "qwen3:4b", "nomic-embed-text:latest"]);
        let chosen = choose_chat_tag(&tags, &HashMap::new());
        assert_eq!(chosen.as_deref(), Some("qwen2.5-coder:7b"));
    }

    #[test]
    fn embed_only_catalog_adopts_nothing() {
        let tags = tags(&[
            "nomic-embed-text:latest",
            "bge-m3:latest",
            "all-minilm:l6-v2",
            "qllama/bge-reranker-v2-m3:latest",
        ]);
        assert_eq!(choose_chat_tag(&tags, &HashMap::new()), None);
        let catalog = LiveLocalOllamaCatalog {
            endpoint_v1: "http://localhost:11434/v1".into(),
            chat_tag: choose_chat_tag(&tags, &HashMap::new()),
            tags,
        };
        assert_eq!(catalog.preferred_tag(), None);
    }

    #[test]
    fn reported_capabilities_outrank_the_name_heuristic() {
        let tags = tags(&["alpha:1b", "mystery:latest", "zeta:8b"]);
        let mut profiles = HashMap::new();
        // An embedding model with an innocent name is excluded by its facts.
        profiles.insert(
            "alpha:1b".to_string(),
            OllamaTagProfile {
                capabilities: Some(vec!["embedding".into()]),
                context_length: Some(8_192),
            },
        );
        // Tool support is preferred over a larger context without it.
        profiles.insert(
            "mystery:latest".to_string(),
            OllamaTagProfile {
                capabilities: Some(vec!["completion".into(), "tools".into()]),
                context_length: Some(32_768),
            },
        );
        profiles.insert(
            "zeta:8b".to_string(),
            OllamaTagProfile {
                capabilities: Some(vec!["completion".into()]),
                context_length: Some(131_072),
            },
        );
        assert_eq!(
            choose_chat_tag(&tags, &profiles).as_deref(),
            Some("mystery:latest")
        );
    }

    #[test]
    fn larger_context_then_name_breaks_ties_among_equals() {
        let tags = tags(&["b-model:7b", "a-model:7b", "c-model:7b"]);
        let mut profiles = HashMap::new();
        profiles.insert(
            "c-model:7b".to_string(),
            OllamaTagProfile {
                capabilities: Some(vec!["completion".into()]),
                context_length: Some(65_536),
            },
        );
        assert_eq!(
            choose_chat_tag(&tags, &profiles).as_deref(),
            Some("c-model:7b")
        );
        assert_eq!(
            choose_chat_tag(&tags, &HashMap::new()).as_deref(),
            Some("a-model:7b"),
            "without facts the first chat tag wins, as before"
        );
    }

    #[test]
    fn parse_ollama_show_response_reads_capabilities_and_context() {
        let body = r#"{
            "capabilities": ["completion", "tools"],
            "model_info": {"general.architecture": "qwen2", "qwen2.context_length": 32768}
        }"#;
        let profile = parse_ollama_show_response(body).expect("parse");
        assert_eq!(
            profile.capabilities,
            Some(vec!["completion".to_string(), "tools".to_string()])
        );
        assert_eq!(profile.context_length, Some(32_768));
        let legacy = parse_ollama_show_response(r#"{"modelfile":""}"#).expect("parse");
        assert_eq!(legacy, OllamaTagProfile::default());
    }

    #[tokio::test]
    async fn probe_live_local_ollama_catalog_reads_api_tags() {
        let _lock = lock_test_env();
        let _live = crate::provider_lake::lock_live_snapshot();
        let home = tempfile::tempdir().unwrap();
        let _home = EnvVarGuard::set("CODEWHALE_HOME", home.path());
        crate::provider_catalog_live::reset_cache_for_test();
        crate::provider_lake::clear_live_snapshot();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            use std::io::{Read, Write};
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let body = br#"{"models":[{"name":"qwen2.5:0.5b","model":"qwen2.5:0.5b","capabilities":["completion"]}]}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        });

        let endpoint = format!("http://{addr}/v1");
        let mut config = Config::default();
        config
            .provider_config_for_mut(&config.test_identity_for_kind(ProviderKind::Ollama))
            .unwrap()
            .base_url = Some(endpoint.clone());

        let catalog = probe_live_local_ollama_catalog(&config)
            .await
            .expect("tags probe should succeed");
        assert_eq!(catalog.endpoint_v1, endpoint);
        assert_eq!(catalog.tags, vec!["qwen2.5:0.5b".to_string()]);
        assert_eq!(catalog.preferred_tag(), Some("qwen2.5:0.5b"));
    }
}
