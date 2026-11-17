//! Topic names (ADR 0005): `channel:<platform>:<id>`, `set:<id>`, `user:<platform>:<id>`, and
//! `cosmetics` for the cosmetics catalog (ADR 0015).

/// The topic badge and paint changes go out on.
pub const COSMETICS: &str = cyex_tv_db::outbox::COSMETICS_TOPIC;

/// Topics one connection or subscription may hold.
pub const MAX_TOPICS: usize = 500;

/// Why a topic was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicError {
    Invalid,
    TooMany,
}

impl TopicError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Invalid => "invalid_topic",
            Self::TooMany => "too_many_topics",
        }
    }
}

fn platform_id(rest: &str) -> bool {
    match rest.split_once(':') {
        Some(("twitch" | "kick", id)) => {
            !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
        }
        _ => false,
    }
}

/// Whether `topic` is a well-formed topic name.
pub fn is_valid(topic: &str) -> bool {
    if topic == COSMETICS {
        return true;
    }
    if let Some(rest) = topic.strip_prefix("channel:") {
        return platform_id(rest);
    }
    if let Some(rest) = topic.strip_prefix("user:") {
        return platform_id(rest);
    }
    if let Some(id) = topic.strip_prefix("set:") {
        return cyex_tv_db::ids::from_text(id).is_some()
            && id.len() == 26
            && id
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topics_are_checked() {
        let set = cyex_tv_db::ids::to_text(cyex_tv_db::ids::new_id());
        for ok in [
            "channel:twitch:123".to_owned(),
            "channel:kick:9".to_owned(),
            "user:twitch:1".to_owned(),
            format!("set:{set}"),
            "cosmetics".to_owned(),
        ] {
            assert!(is_valid(&ok), "{ok}");
        }
        for bad in [
            "channel:youtube:1".to_owned(),
            "channel:twitch:".to_owned(),
            "channel:twitch:12a".to_owned(),
            "channel:twitch:123456789012345678901".to_owned(),
            "user:kick".to_owned(),
            "set:nope".to_owned(),
            format!("set:{}", set.to_lowercase()),
            "cosmetic:badge:1".to_owned(),
            "cosmetics:".to_owned(),
            "Cosmetics".to_owned(),
            String::new(),
        ] {
            assert!(!is_valid(&bad), "{bad}");
        }
        assert_eq!(TopicError::TooMany.code(), "too_many_topics");
    }
}
