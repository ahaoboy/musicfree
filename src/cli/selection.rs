use crate::Playlist;
use anyhow::{Context, Result, bail};

pub fn apply(playlist: &mut Playlist, spec: &str) -> Result<()> {
    let indices = parse(spec)?;
    let default_id = playlist
        .default
        .and_then(|index| playlist.tracks.get(index))
        .map(|track| track.id.clone());
    let total = playlist.tracks.len();
    let selected = indices
        .into_iter()
        .filter_map(|index| {
            if index > total {
                eprintln!("warning: item {index} is out of range (playlist has {total} items)");
                None
            } else {
                Some(playlist.tracks[index - 1].clone())
            }
        })
        .collect::<Vec<_>>();

    if selected.is_empty() {
        bail!("no valid playlist items selected");
    }

    playlist.tracks = selected;
    playlist.default =
        default_id.and_then(|id| playlist.tracks.iter().position(|track| track.id == id));
    Ok(())
}

pub fn parse(spec: &str) -> Result<Vec<usize>> {
    let mut items = Vec::new();

    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            bail!("empty playlist item");
        }
        let (start, end) = match part.split_once('-') {
            Some((start, end)) => (number(start)?, number(end)?),
            None => {
                let item = number(part)?;
                (item, item)
            }
        };
        if start > end {
            bail!("invalid range {start}-{end}");
        }
        items.extend(start..=end);
    }

    items.sort_unstable();
    items.dedup();
    Ok(items)
}

fn number(value: &str) -> Result<usize> {
    let value = value
        .trim()
        .parse()
        .with_context(|| format!("invalid item number '{}'", value.trim()))?;
    if value == 0 {
        bail!("playlist items start at 1");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges_and_deduplicates() {
        assert_eq!(parse("1,3-5,4").unwrap(), vec![1, 3, 4, 5]);
    }

    #[test]
    fn rejects_zero() {
        assert!(parse("0").is_err());
    }
}
