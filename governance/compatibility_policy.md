# Compatibility Policy

Date: 2026-03-01
Status: active

## Intent

This project prioritizes domain clarity and ownership over backward compatibility.

## Rules

- Backward incompatible changes are allowed when they improve domain clarity and ownership.
- Any backward incompatible change must be called out to the user before commit.
- Commit messages must reflect change severity using conventional commit rules.
- Use `type!:` or `type(scope)!:` for breaking changes.
- Add a `BREAKING CHANGE:` footer with a concise migration impact note.
- Keep user facing impact explicit in pull request notes review notes and release notes.
