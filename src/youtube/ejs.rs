use std::collections::HashMap;

use crate::error::{MusicFreeError, Result};
use url::Url;
use ytdlp_ejs::{
    JsChallengeInput, JsChallengeOutput, JsChallengeRequest, JsChallengeResponse, JsChallengeType,
    RuntimeType,
};

/// Get the appropriate runtime type based on enabled features
const fn get_runtime_type() -> RuntimeType {
    #[cfg(feature = "qjs")]
    let rt = RuntimeType::QuickJS;

    #[cfg(all(not(feature = "qjs"), feature = "boa"))]
    let rt = RuntimeType::Boa;

    rt
}
/// Extract 'n' parameter from URL
fn extract_n_param(url: &Url) -> Result<String> {
    url.query_pairs()
        .find(|(k, _)| k == "n")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| {
            MusicFreeError::CipherParseError("Parameter 'n' not found in URL".to_string())
        })
}

pub(crate) fn solve_n(url_str: &str, player: String) -> Result<String> {
    let url_obj = Url::parse(url_str)
        .map_err(|e| MusicFreeError::CipherParseError(format!("Failed to parse URL: {}", e)))?;

    // Extract n parameter from URL
    let n = extract_n_param(&url_obj)?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] solve_n: input n={n}");

    // Execute JS challenge for n parameter
    let results = execute_js_challenges(player, vec![(JsChallengeType::N, vec![n.clone()])])?;

    // Get the transformed n value
    let new_n = results.get(&n).ok_or_else(|| {
        MusicFreeError::JsDecryptionFailed(
            "Failed to get valid response for n parameter".to_string(),
        )
    })?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] solve_n: output n={new_n}");

    // Update URL with new n parameter
    update_url_query(url_obj, &["n"], &[("n".to_string(), new_n.clone())])
}

/// Update URL query parameters, removing specified keys and adding new ones
fn update_url_query(
    mut url: Url,
    remove_keys: &[&str],
    add_params: &[(String, String)],
) -> Result<String> {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !remove_keys.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    pairs.extend(add_params.iter().cloned());
    url.query_pairs_mut().clear().extend_pairs(pairs);
    Ok(url.to_string())
}

pub(crate) fn solve_cipher(cipher_str: &str, player: String) -> Result<String> {
    // Parse signatureCipher parameters
    let cipher_params: HashMap<String, String> = url::form_urlencoded::parse(cipher_str.as_bytes())
        .into_owned()
        .collect();

    let url_str = cipher_params
        .get("url")
        .ok_or_else(|| MusicFreeError::CipherParseError("Missing url in cipher".to_string()))?;
    let sp = cipher_params.get("sp").map(|s| s.as_str()).unwrap_or("sig");
    let s = cipher_params
        .get("s")
        .ok_or_else(|| MusicFreeError::CipherParseError("Missing s in cipher".to_string()))?;

    // Parse URL and extract 'n' parameter
    let url_obj = Url::parse(url_str)
        .map_err(|e| MusicFreeError::CipherParseError(format!("Failed to parse URL: {}", e)))?;
    let n = extract_n_param(&url_obj)?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] solve_cipher: s={s}, n={n}, sp={sp}");

    // Execute JS challenges for both n and s parameters
    let results = execute_js_challenges(
        player,
        vec![
            (JsChallengeType::N, vec![n.clone()]),
            (JsChallengeType::Sig, vec![s.to_string()]),
        ],
    )?;

    // Extract transformed values
    let new_n = results.get(&n).ok_or_else(|| {
        MusicFreeError::JsDecryptionFailed("Failed to decrypt n parameter".to_string())
    })?;
    let new_sig = results.get(s).ok_or_else(|| {
        MusicFreeError::JsDecryptionFailed("Failed to decrypt s parameter".to_string())
    })?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] solve_cipher: new_sig={new_sig}, new_n={new_n}");

    // Update URL with new parameters
    update_url_query(
        url_obj,
        &["n", sp],
        &[
            ("n".to_string(), new_n.clone()),
            (sp.to_string(), new_sig.clone()),
        ],
    )
}

/// Execute JS challenges and return response data
fn execute_js_challenges(
    player: String,
    challenges: Vec<(JsChallengeType, Vec<String>)>,
) -> Result<HashMap<String, String>> {
    let requests = challenges
        .into_iter()
        .map(|(challenge_type, challenges_vec)| JsChallengeRequest {
            challenge_type,
            challenges: challenges_vec,
        })
        .collect();

    let input = JsChallengeInput::Player {
        player,
        requests,
        output_preprocessed: false,
    };

    let runtime_type = get_runtime_type();

    #[cfg(debug_assertions)]
    {
        eprintln!("[debug] EJS runtime: {runtime_type:?}");
        if let JsChallengeInput::Player { ref player, .. } = input {
            let preview: String = player.chars().take(100).collect();
            eprintln!(
                "[debug] EJS player JS size: {} bytes, preview: {preview}...",
                player.len()
            );
        }
    }

    #[cfg(feature = "dev")]
    if let JsChallengeInput::Player { player, .. } = &input {
        let path = std::path::Path::new("player.js");
        if let Err(e) = std::fs::write(path, player) {
            eprintln!("[dev] Failed to write player.js: {e}");
        } else {
            eprintln!("[dev] Wrote player.js ({} bytes)", player.len());
        }
    }

    let output = ytdlp_ejs::process_input(input, runtime_type);

    #[cfg(debug_assertions)]
    match &output {
        JsChallengeOutput::Result { responses, .. } => {
            for (i, resp) in responses.iter().enumerate() {
                match resp {
                    JsChallengeResponse::Result { data } => {
                        eprintln!("[debug] EJS response[{i}]: {data:?}");
                    }
                    JsChallengeResponse::Error { error } => {
                        eprintln!("[debug] EJS response[{i}] ERROR: {error}");
                    }
                }
            }
        }
        JsChallengeOutput::Error { error } => {
            eprintln!("[debug] EJS output ERROR: {error}");
        }
    }

    match output {
        JsChallengeOutput::Result { responses, .. } => {
            let mut results = HashMap::new();
            for response in responses {
                if let JsChallengeResponse::Result { data } = response {
                    results.extend(data);
                }
            }
            Ok(results)
        }
        JsChallengeOutput::Error { error } => Err(MusicFreeError::JsDecryptionFailed(format!(
            "JS execution failed: {}",
            error
        ))),
    }
}
