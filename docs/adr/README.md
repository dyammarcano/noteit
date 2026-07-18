# Architecture Decision Records

Each ADR captures one significant decision: its context, the choice made, and
the consequences. ADRs are point-in-time records — they track status via the
`Status` field rather than a revision tag, and are not edited in place except to
mark one `Superseded`.

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-repo-identity-from-root-commit.md) | Repo identity from the root commit SHA | Accepted |
| [0002](0002-no-clap-ambiguity-rule.md) | No `clap`; hand-rolled ambiguity rule | Accepted |
| [0003](0003-single-sqlite-store-with-fts5.md) | Single SQLite store with FTS5 | Accepted |
| [0004](0004-std-only-plugin-host-contract.md) | Std-only ported plugin-host contract | Accepted |
| [0005](0005-hard-delete-exception.md) | Hard delete as the one note-losing exception | Accepted |
| [0006](0006-cli-stability-pre-1.0.md) | CLI stability & deprecation stance (pre-1.0) | Accepted |
