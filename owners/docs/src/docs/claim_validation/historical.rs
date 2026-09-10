//! Frozen interpretation of retained unversioned README captures.
//! Read-only compatibility for existing owner history; never selected for new captures.
//! Remove only when retained observations without an extraction selection are migrated
//! or no longer supported. Historical claim IDs and capture digests depend on this grammar.

use super::{ClaimKind, ReadmeClaim};
use std::collections::BTreeSet;

/// Deterministically enumerate all assertion-bearing Markdown blocks.
pub(crate) fn extract_historical_claims(path: &str, content: &str) -> Vec<ReadmeClaim> {
    let mut claims = Vec::new();
    let mut paragraph = Vec::<(usize, String)>::new();
    let mut in_code_block = false;
    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(path, &mut paragraph, &mut claims);
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            if !trimmed.is_empty() {
                push_claim(
                    path,
                    trimmed.to_string(),
                    ClaimKind::CodeLine,
                    line_number,
                    line_number,
                    &mut claims,
                );
            }
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(path, &mut paragraph, &mut claims);
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            flush_paragraph(path, &mut paragraph, &mut claims);
            push_claim(
                path,
                title.trim().to_string(),
                ClaimKind::Title,
                line_number,
                line_number,
                &mut claims,
            );
            continue;
        }
        if trimmed.starts_with('#') || is_table_separator(trimmed) {
            flush_paragraph(path, &mut paragraph, &mut claims);
            continue;
        }
        if let Some(item) = strip_list_marker(trimmed) {
            flush_paragraph(path, &mut paragraph, &mut claims);
            push_claim(
                path,
                item.to_string(),
                ClaimKind::ListItem,
                line_number,
                line_number,
                &mut claims,
            );
            continue;
        }
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            flush_paragraph(path, &mut paragraph, &mut claims);
            push_claim(
                path,
                trimmed.to_string(),
                ClaimKind::TableRow,
                line_number,
                line_number,
                &mut claims,
            );
            continue;
        }
        paragraph.push((
            line_number,
            trimmed.trim_start_matches('>').trim().to_string(),
        ));
    }
    flush_paragraph(path, &mut paragraph, &mut claims);
    claims
}

fn flush_paragraph(
    path: &str,
    paragraph: &mut Vec<(usize, String)>,
    claims: &mut Vec<ReadmeClaim>,
) {
    if paragraph.is_empty() {
        return;
    }
    let start = paragraph.first().unwrap().0;
    let end = paragraph.last().unwrap().0;
    let statement = paragraph
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    paragraph.clear();
    push_claim(path, statement, ClaimKind::Prose, start, end, claims);
}

fn push_claim(
    path: &str,
    statement: String,
    kind: ClaimKind,
    start: usize,
    end: usize,
    claims: &mut Vec<ReadmeClaim>,
) {
    if statement.trim().is_empty() {
        return;
    }
    let seed = format!("{path}::{start}::{end}::{kind:?}::{statement}");
    claims.push(ReadmeClaim {
        claim_id: format!("claim-{}", blake3::hash(seed.as_bytes()).to_hex()),
        literal_requirements: literal_requirements(&statement),
        statement,
        kind,
        source_line_start: start,
        source_line_end: end,
    });
}

fn literal_requirements(statement: &str) -> Vec<String> {
    let mut literals = BTreeSet::new();
    let mut remaining = statement;
    while let Some(start) = remaining.find('`') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let literal = &after_start[..end];
        if !literal.trim().is_empty() {
            literals.insert(literal.to_string());
        }
        remaining = &after_start[end + 1..];
    }
    for token in statement.split_whitespace() {
        let candidate = token.trim_matches(|character: char| {
            matches!(
                character,
                '(' | ')' | '[' | ']' | '<' | '>' | ',' | '.' | ';' | ':' | '"' | '\''
            )
        });
        if candidate.starts_with("https://") || candidate.starts_with("http://") {
            literals.insert(candidate.to_string());
        }
    }
    literals.into_iter().collect()
}

fn strip_list_marker(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(item) = line.strip_prefix(marker) {
            return Some(item.trim());
        }
    }
    let digits = line
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 {
        return line
            .get(digits..)
            .and_then(|rest| rest.strip_prefix(". "))
            .map(str::trim);
    }
    None
}

fn is_table_separator(line: &str) -> bool {
    line.starts_with('|')
        && line.ends_with('|')
        && line
            .chars()
            .all(|character| matches!(character, '|' | '-' | ':' | ' '))
}
