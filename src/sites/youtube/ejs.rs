use crate::{Error, Result};
use std::collections::HashMap;
use url::Url;
use ytdlp_ejs::{
    JsChallengeInput, JsChallengeOutput, JsChallengeRequest, JsChallengeResponse, JsChallengeType,
    RuntimeType,
};

pub fn solve_n(url: &str, player_js: &str) -> Result<String> {
    let url = Url::parse(url).map_err(|error| Error::Cipher(error.to_string()))?;
    let n = query_value(&url, "n")?;
    let results = execute(player_js, vec![(JsChallengeType::N, vec![n.clone()])])?;
    let n = results
        .get(&n)
        .ok_or_else(|| Error::Js("no result for n challenge".into()))?;
    Ok(replace_query(url, &["n"], &[("n", n)]))
}

pub fn solve_cipher(cipher: &str, player_js: &str) -> Result<String> {
    let params = url::form_urlencoded::parse(cipher.as_bytes())
        .into_owned()
        .collect::<HashMap<_, _>>();
    let url = params
        .get("url")
        .ok_or_else(|| Error::Cipher("missing url in cipher".into()))?;
    let signature = params
        .get("s")
        .ok_or_else(|| Error::Cipher("missing s in cipher".into()))?;
    let signature_param = params.get("sp").map_or("sig", String::as_str);
    let parsed = Url::parse(url).map_err(|error| Error::Cipher(error.to_string()))?;
    let n = query_value(&parsed, "n").ok();

    let mut challenges = vec![(JsChallengeType::Sig, vec![signature.clone()])];
    if let Some(n) = &n {
        challenges.push((JsChallengeType::N, vec![n.clone()]));
    }
    let results = execute(player_js, challenges)?;
    let signature = results
        .get(signature)
        .ok_or_else(|| Error::Js("no result for signature challenge".into()))?;

    let remove = vec!["n", signature_param];
    let mut add = vec![(signature_param, signature.as_str())];
    if let Some(n) = n {
        let value = results
            .get(&n)
            .ok_or_else(|| Error::Js("no result for n challenge".into()))?;
        add.push(("n", value));
    }

    Ok(replace_query(parsed, &remove, &add))
}

fn query_value(url: &Url, key: &str) -> Result<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| Error::Cipher(format!("missing '{key}' parameter")))
}

fn replace_query(url: Url, remove: &[&str], add: &[(&str, &str)]) -> String {
    let mut url = url;
    let mut pairs = url
        .query_pairs()
        .filter(|(key, _)| !remove.contains(&key.as_ref()))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    pairs.extend(
        add.iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
    );
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(pairs);
    url.to_string()
}

fn execute(
    player_js: &str,
    challenges: Vec<(JsChallengeType, Vec<String>)>,
) -> Result<HashMap<String, String>> {
    let requests = challenges
        .into_iter()
        .map(|(challenge_type, challenges)| JsChallengeRequest {
            challenge_type,
            challenges,
        })
        .collect();
    let input = JsChallengeInput::Player {
        player: player_js.to_owned(),
        requests,
        output_preprocessed: false,
    };
    let runtime = RuntimeType::default_runtime()
        .ok_or_else(|| Error::Js("no JavaScript runtime available".into()))?;

    let output = std::thread::Builder::new()
        .name("ejs".into())
        .stack_size(128 * 1024 * 1024)
        .spawn(move || ytdlp_ejs::process_input(input, runtime))
        .map_err(|error| Error::Js(format!("failed to spawn JS thread: {error}")))?
        .join()
        .map_err(|_| Error::Js("JS thread panicked".into()))?;

    match output {
        JsChallengeOutput::Result { responses, .. } => {
            let mut results = HashMap::new();
            for response in responses {
                match response {
                    JsChallengeResponse::Result { data } => results.extend(data),
                    JsChallengeResponse::Error { error } => return Err(Error::Js(error)),
                }
            }
            Ok(results)
        }
        JsChallengeOutput::Error { error } => Err(Error::Js(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_query_values() {
        let url = Url::parse("https://example.com/a?n=old&x=1").unwrap();
        assert_eq!(
            replace_query(url, &["n"], &[("n", "new")]),
            "https://example.com/a?x=1&n=new"
        );
    }
}
