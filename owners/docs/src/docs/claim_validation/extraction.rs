//! Selected README reading grammar. Blocks preserve document context; sentence
//! candidates do not claim that every sentence is factual or semantically atomic.

use super::{ClaimKind, ReadmeClaim};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsClaimExtraction {
    MarkdownSentencesV2,
}

/// Enumerate candidates without rewriting the document. The selected semantic
/// judge distinguishes factual assertions from navigation and other nonassertive text.
pub fn extract_selected_claims(
    selection: DocsClaimExtraction,
    path: &str,
    content: &str,
) -> Vec<ReadmeClaim> {
    match selection {
        DocsClaimExtraction::MarkdownSentencesV2 => markdown_sentences(path, content),
    }
}

fn markdown_sentences(path: &str, content: &str) -> Vec<ReadmeClaim> {
    let mut claims = Vec::new();
    let mut paragraph = Vec::new();
    let mut fence = None;
    for (index, raw) in content.lines().enumerate() {
        let line = index + 1;
        let text = raw.trim();
        let marker = text.chars().next().filter(|c| matches!(c, '`' | '~'));
        let count = marker.map_or(0, |c| text.chars().take_while(|x| *x == c).count());
        if let Some((character, length)) = fence {
            if marker == Some(character) && count >= length && text[count..].trim().is_empty() {
                fence = None;
            } else if !text.is_empty() {
                emit(path, text, ClaimKind::CodeLine, line, line, &mut claims);
            }
            continue;
        }
        if count >= 3 {
            flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
            fence = Some((marker.unwrap(), count));
            continue;
        }
        if text.is_empty() {
            flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
            continue;
        }
        // Setext heading underlines belong to the preceding text, not assertions.
        if text.chars().all(|c| c == '=') || text.len() >= 3 && text.chars().all(|c| c == '-') {
            flush(path, &mut paragraph, ClaimKind::Title, &mut claims);
            continue;
        }
        let heading = text.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&heading) && text[heading..].starts_with(char::is_whitespace) {
            flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
            emit(
                path,
                text[heading..].trim().trim_end_matches('#').trim_end(),
                ClaimKind::Title,
                line,
                line,
                &mut claims,
            );
            continue;
        }
        if text.starts_with('|') && text.ends_with('|') {
            flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
            if !text.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                emit(path, text, ClaimKind::TableRow, line, line, &mut claims);
            }
            continue;
        }
        let item = ["- ", "* ", "+ "]
            .iter()
            .find_map(|marker| text.strip_prefix(marker))
            .or_else(|| {
                let digits = text.chars().take_while(char::is_ascii_digit).count();
                (digits > 0)
                    .then(|| {
                        text[digits..]
                            .strip_prefix(". ")
                            .or_else(|| text[digits..].strip_prefix(") "))
                    })
                    .flatten()
            });
        if let Some(item) = item {
            flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
            emit_sentences(
                path,
                &[(line, item.to_string())],
                ClaimKind::ListItem,
                &mut claims,
            );
        } else {
            paragraph.push((line, text.trim_start_matches('>').trim().to_string()));
        }
    }
    flush(path, &mut paragraph, ClaimKind::Prose, &mut claims);
    claims
}

fn flush(
    path: &str,
    lines: &mut Vec<(usize, String)>,
    kind: ClaimKind,
    claims: &mut Vec<ReadmeClaim>,
) {
    emit_sentences(path, lines, kind, claims);
    lines.clear();
}

fn emit_sentences(
    path: &str,
    lines: &[(usize, String)],
    kind: ClaimKind,
    claims: &mut Vec<ReadmeClaim>,
) {
    let text = lines
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut start = 0;
    let mut code = false;
    let mut brackets = 0usize;
    let mut ranges = Vec::new();
    for (offset, character) in text.char_indices() {
        match character {
            '`' => code = !code,
            '[' | '(' if !code => brackets += 1,
            ']' | ')' if !code => brackets = brackets.saturating_sub(1),
            '.' | '!' | '?' if !code && brackets == 0 => {
                let end = offset + character.len_utf8();
                if text[end..].is_empty() || text[end..].starts_with(char::is_whitespace) {
                    ranges.push(start..end);
                    start = end;
                }
            }
            _ => {}
        }
    }
    ranges.push(start..text.len());
    for range in ranges {
        let statement = text[range.clone()].trim();
        if statement.is_empty() {
            continue;
        }
        let begin =
            range.start + text[range.clone()].len() - text[range.clone()].trim_start().len();
        let end = begin + statement.len();
        let mut offset = 0;
        let mut touched = Vec::new();
        for (line, text) in lines {
            if offset < end && offset + text.len() > begin {
                touched.push(*line);
            }
            offset += text.len() + 1;
        }
        emit(
            path,
            statement,
            kind,
            touched[0],
            *touched.last().unwrap(),
            claims,
        );
    }
}

fn emit(
    path: &str,
    statement: &str,
    kind: ClaimKind,
    start: usize,
    end: usize,
    claims: &mut Vec<ReadmeClaim>,
) {
    if statement.is_empty() {
        return;
    }
    // Include the occurrence ordinal so identical assertions on one source line
    // remain separately accountable. V1 identities remain in the historical reader.
    let seed = format!(
        "markdown_sentences_v2::{path}::{start}::{end}::{kind:?}::{}::{statement}",
        claims.len()
    );
    let literals = statement
        .split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, value)| value.to_string())
        .chain(url_literals(statement))
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    claims.push(ReadmeClaim {
        claim_id: format!("claim-{}", blake3::hash(seed.as_bytes()).to_hex()),
        statement: statement.into(),
        kind,
        source_line_start: start,
        source_line_end: end,
        literal_requirements: literals.into_iter().collect(),
    });
}

fn url_literals(statement: &str) -> Vec<String> {
    ["https://", "http://"]
        .into_iter()
        .flat_map(|scheme| {
            statement.match_indices(scheme).map(|(start, _)| {
                let tail = &statement[start..];
                let mut depth = 0usize;
                let end = tail
                    .char_indices()
                    .find_map(|(index, character)| {
                        match character {
                            '(' => depth += 1,
                            ')' if depth > 0 => depth -= 1,
                            ')' | '<' | '>' | ']' | '"' | '\'' => return Some(index),
                            value if value.is_whitespace() => return Some(index),
                            _ => {}
                        }
                        None
                    })
                    .unwrap_or(tail.len());
                tail[..end]
                    .trim_end_matches(['.', ',', ';', '!', '?'])
                    .to_string()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrative_sentences_and_all_heading_levels_keep_exact_locations() {
        let content = "# Tool\n\nIt reads input. It writes output\non demand.\n\n## Usage\n\n### It accepts UTF-8\n\n~~~rust\nrun();\n~~~\n";
        let claims = extract_selected_claims(
            DocsClaimExtraction::MarkdownSentencesV2,
            "README.md",
            content,
        );
        assert_eq!(
            claims
                .iter()
                .map(|claim| claim.statement.as_str())
                .collect::<Vec<_>>(),
            [
                "Tool",
                "It reads input.",
                "It writes output on demand.",
                "Usage",
                "It accepts UTF-8",
                "run();"
            ]
        );
        assert_eq!(
            (claims[2].source_line_start, claims[2].source_line_end),
            (3, 4)
        );
        assert_eq!(claims.last().unwrap().kind, ClaimKind::CodeLine);
    }

    #[test]
    fn repeated_sentences_inline_code_and_link_destinations_are_not_lost() {
        let claims = markdown_sentences(
            "README.md",
            "It runs. It runs. Use `x. y` and [guide](https://example.test/a).\n",
        );
        assert_eq!(claims.len(), 3);
        assert_ne!(claims[0].claim_id, claims[1].claim_id);
        assert_eq!(
            claims[2].literal_requirements,
            vec!["https://example.test/a", "x. y"]
        );
        assert_eq!(
            url_literals("[guide](https://example.test/a(b))."),
            vec!["https://example.test/a(b)"]
        );
        assert_eq!(
            url_literals("<https://example.test/a> and http://example.test/b."),
            vec!["https://example.test/a", "http://example.test/b"]
        );
    }
}
