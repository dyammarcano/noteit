# fish completion for noteit — mirrors VERBS in src/cli.rs.
set -l verbs add list search new done open project adopt delete plugin

# Top-level verbs (only when no verb yet).
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a add -d "capture verb-colliding text"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a list -d "list notes"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a search -d "full-text search"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a new -d "note in \$EDITOR"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a done -d "mark done"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a open -d "reopen"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a project -d "rename project"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a adopt -d "undo adoption"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a delete -d "delete a note"
complete -c noteit -n "not __fish_seen_subcommand_from $verbs" -f -a plugin -d "install into an AI host"

# plugin subcommands + --host.
complete -c noteit -n "__fish_seen_subcommand_from plugin" -f -a "list install status doctor uninstall"
complete -c noteit -n "__fish_seen_subcommand_from plugin" -l host -f -a "claude codex gemini all"

# list/search flags.
complete -c noteit -n "__fish_seen_subcommand_from list" -l global -l flat -l all -l tag -l limit
complete -c noteit -n "__fish_seen_subcommand_from search" -l global
complete -c noteit -n "__fish_seen_subcommand_from adopt" -l undo
