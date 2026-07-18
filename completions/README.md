# Shell completions

Static completion scripts for `noteit`. They mirror the verb/flag set in
`src/cli.rs` (`VERBS`); update them when a verb or flag is added.

| Shell | File | Install |
|-------|------|---------|
| bash | `noteit.bash` | source it from `~/.bashrc`, or drop in `/etc/bash_completion.d/` |
| zsh | `_noteit` | put on your `$fpath` (e.g. `~/.zsh/completions/`), then `compinit` |
| fish | `noteit.fish` | copy to `~/.config/fish/completions/` |
| PowerShell | `noteit.ps1` | dot-source it from your `$PROFILE` |

Example (bash): `echo 'source /path/to/completions/noteit.bash' >> ~/.bashrc`
