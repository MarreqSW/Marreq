// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! `@username` mentions in comment bodies.
//!
//! A mention is `@` followed by a username (`[A-Za-z0-9_]`, 3–50 characters, the
//! same rule user validation applies). The `@` must start the text or follow a
//! character that is neither a word character nor `@`, so e-mail addresses such as
//! `alice@example.com` are not mentions. The frontend mirrors this rule in
//! `frontend/src/utils/mentions.ts`.

const MIN_LEN: usize = 3;
const MAX_LEN: usize = 50;

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Usernames mentioned in `body`, lower-cased and de-duplicated, in order of first
/// appearance.
pub fn extract_mentions(body: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut prev: Option<char> = None;
    for (i, c) in body.char_indices() {
        // Any letter before the `@`, ASCII or not, makes it part of a word.
        let boundary = prev.is_none_or(|p| !p.is_alphanumeric() && p != '_' && p != '@');
        if c == '@' && boundary {
            let start = i + 1;
            let end = body[start..]
                .find(|ch: char| !is_word(ch))
                .map_or(body.len(), |n| start + n);
            let next = body[end..].chars().next();
            let len = end - start;
            if (MIN_LEN..=MAX_LEN).contains(&len) && !next.is_some_and(char::is_alphanumeric) {
                let name = body[start..end].to_ascii_lowercase();
                if !found.contains(&name) {
                    found.push(name);
                }
            }
        }
        prev = Some(c);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::extract_mentions;

    #[test]
    fn finds_mentions_at_start_middle_and_end() {
        assert_eq!(
            extract_mentions("@bob please ask @carol, then @dave"),
            vec!["bob", "carol", "dave"]
        );
    }

    #[test]
    fn accepts_punctuation_around_mentions() {
        assert_eq!(
            extract_mentions("(@bob) @carol: @dave.\n@erin!"),
            vec!["bob", "carol", "dave", "erin"]
        );
    }

    #[test]
    fn ignores_email_addresses_and_double_at() {
        assert!(extract_mentions("mail alice@example.com or @@bob").is_empty());
    }

    #[test]
    fn ignores_too_short_and_too_long_names() {
        let long = format!("@{}", "a".repeat(51));
        assert!(extract_mentions(&format!("@ab @ {long}")).is_empty());
        assert_eq!(extract_mentions(&format!("@{}", "a".repeat(50))).len(), 1);
    }

    #[test]
    fn deduplicates_case_insensitively() {
        assert_eq!(extract_mentions("@Bob @bob @BOB"), vec!["bob"]);
    }

    #[test]
    fn handles_non_ascii_text() {
        assert_eq!(
            extract_mentions("Gràcies @jordi — ñ@bob @añé"),
            vec!["jordi"]
        );
    }
}
