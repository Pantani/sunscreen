//! Embedded learning topics.

use rust_embed::RustEmbed;

use crate::error::SunscreenError;
use crate::onboarding::args::LearnArgs;

#[derive(RustEmbed)]
#[folder = "assets/learn/"]
struct LearnAssets;

#[derive(Debug, Clone)]
struct Topic {
    slug: &'static str,
    title: &'static str,
    est_minutes: u8,
}

const TOPICS: &[Topic] = &[
    Topic {
        slug: "pda",
        title: "Program Derived Addresses",
        est_minutes: 6,
    },
    Topic {
        slug: "cpi",
        title: "Cross-Program Invocation",
        est_minutes: 7,
    },
    Topic {
        slug: "token-2022",
        title: "Token-2022",
        est_minutes: 6,
    },
    Topic {
        slug: "accounts-model",
        title: "Anchor Accounts Model",
        est_minutes: 5,
    },
    Topic {
        slug: "anchor-vs-native",
        title: "Anchor vs Native Solana",
        est_minutes: 5,
    },
];

pub fn run(args: &LearnArgs, json: bool) -> Result<i32, SunscreenError> {
    match args.topic.as_deref() {
        Some("list") | None => list_topics(json),
        Some(topic) => render_topic(topic, json),
    }
}

fn list_topics(json: bool) -> Result<i32, SunscreenError> {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "topics": TOPICS.iter().map(topic_json).collect::<Vec<_>>(),
            })
        );
    } else {
        for topic in TOPICS {
            println!("{}\t{}\t{} min", topic.slug, topic.title, topic.est_minutes);
        }
    }
    Ok(0)
}

fn render_topic(slug: &str, json: bool) -> Result<i32, SunscreenError> {
    let topic = find_topic(slug)?;
    let raw = read_topic(topic.slug)?;
    let parsed = parse_markdown(&raw);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "topic": topic.slug,
                "title": parsed.title.unwrap_or(topic.title.to_string()),
                "est_minutes": parsed.est_minutes.unwrap_or(topic.est_minutes),
                "prereqs": parsed.prereqs,
                "body": parsed.body,
            })
        );
    } else {
        print!("{}", parsed.body);
    }
    Ok(0)
}

fn find_topic(slug: &str) -> Result<&'static Topic, SunscreenError> {
    TOPICS
        .iter()
        .find(|topic| topic.slug == slug)
        .ok_or_else(|| {
            SunscreenError::UserInput(format!(
                "unknown learn topic `{slug}`; run `sunscreen learn`"
            ))
        })
}

fn read_topic(slug: &str) -> Result<String, SunscreenError> {
    let path = format!("{slug}.md");
    let asset = LearnAssets::get(&path).ok_or_else(|| {
        SunscreenError::Other(anyhow::anyhow!("embedded learn topic missing {path}"))
    })?;
    String::from_utf8(asset.data.into_owned())
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("decode {path}: {err}")))
}

#[derive(Debug)]
struct ParsedTopic {
    title: Option<String>,
    est_minutes: Option<u8>,
    prereqs: Vec<String>,
    body: String,
}

fn parse_markdown(raw: &str) -> ParsedTopic {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let Some((frontmatter, body)) = split_frontmatter(&normalized) else {
        return ParsedTopic {
            title: None,
            est_minutes: None,
            prereqs: Vec::new(),
            body: normalized,
        };
    };
    let mut title = None;
    let mut est_minutes = None;
    let mut prereqs = Vec::new();
    for line in frontmatter.lines() {
        if let Some(value) = line.strip_prefix("title:") {
            title = Some(value.trim().trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("est_minutes:") {
            est_minutes = value.trim().parse::<u8>().ok();
        } else if let Some(value) = line.strip_prefix("prereqs:") {
            prereqs = value
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|part| part.trim().trim_matches('"').to_string())
                .filter(|part| !part.is_empty())
                .collect();
        }
    }
    ParsedTopic {
        title,
        est_minutes,
        prereqs,
        body: body.to_string(),
    }
}

fn split_frontmatter(source: &str) -> Option<(&str, &str)> {
    let source = source.trim_start();
    let open_end = source.find('\n').unwrap_or(source.len());
    if source[..open_end].trim() != "---" {
        return None;
    }
    if open_end == source.len() {
        return None;
    }
    let rest = &source[open_end + 1..];
    let mut cursor = 0;
    for line in rest.split_inclusive('\n') {
        let without_newline = line.strip_suffix('\n').unwrap_or(line);
        if without_newline.trim() == "---" {
            let body_start = cursor + line.len();
            return Some((&rest[..cursor], &rest[body_start..]));
        }
        cursor += line.len();
    }
    None
}

fn topic_json(topic: &Topic) -> serde_json::Value {
    serde_json::json!({
        "topic": topic.slug,
        "title": topic.title,
        "est_minutes": topic.est_minutes,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_markdown;

    #[test]
    fn parse_markdown_accepts_crlf_frontmatter() {
        let parsed = parse_markdown(
            "  ---\r\ntitle: \"CRLF Topic\"\r\nest_minutes: 9\r\nprereqs: [\"pda\"]\r\n---\r\n# Body\r\n",
        );

        assert_eq!(parsed.title.as_deref(), Some("CRLF Topic"));
        assert_eq!(parsed.est_minutes, Some(9));
        assert_eq!(parsed.prereqs, vec!["pda"]);
        assert_eq!(parsed.body, "# Body\n");
    }
}
