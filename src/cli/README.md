# CLI Domain

The CLI owns parsing, routing, help, output, and presentation. It does not own domain orchestration or semantic truth.

## Targeting

When a command addresses workspace material and supports both a filesystem path and an opaque internal identity, the filesystem path is the default user-facing target.

Commands that operate on workspace-wide state, Events, runtime sessions, or another naturally non-path scope do not manufacture a path option. Their help and domain contracts state the actual target explicitly.

Parsing may accept compatibility spellings while routing one normalized target into the owning domain service. Compatibility syntax does not create another command authority.

Command tests own the accepted target combinations, mutual exclusions, defaults, and help behavior.
