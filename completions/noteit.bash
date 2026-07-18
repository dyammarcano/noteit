# bash completion for noteit — mirrors VERBS in src/cli.rs.
_noteit() {
    local cur prev verbs plugin_subs hosts
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    verbs="add list search new done open project adopt delete plugin --help --version"
    plugin_subs="list install status doctor uninstall"
    hosts="claude codex gemini all"

    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=( $(compgen -W "$verbs" -- "$cur") )
        return
    fi

    case "${COMP_WORDS[1]}" in
        plugin)
            if [ "$COMP_CWORD" -eq 2 ]; then
                COMPREPLY=( $(compgen -W "$plugin_subs" -- "$cur") )
            elif [ "$prev" = "--host" ]; then
                COMPREPLY=( $(compgen -W "$hosts" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "--host" -- "$cur") )
            fi
            ;;
        list)
            COMPREPLY=( $(compgen -W "--global --flat --tag --all --limit" -- "$cur") )
            ;;
        search)
            COMPREPLY=( $(compgen -W "--global" -- "$cur") )
            ;;
        adopt)
            COMPREPLY=( $(compgen -W "--undo" -- "$cur") )
            ;;
        project)
            COMPREPLY=( $(compgen -W "rename" -- "$cur") )
            ;;
    esac
}
complete -F _noteit noteit
