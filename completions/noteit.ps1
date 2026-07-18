# PowerShell completion for noteit — mirrors VERBS in src/cli.rs.
# Dot-source this file or add it to your $PROFILE.
Register-ArgumentCompleter -Native -CommandName noteit -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $verbs = 'add', 'list', 'search', 'new', 'done', 'open', 'project', 'adopt', 'delete', 'plugin', '--help', '--version'
    $pluginSubs = 'list', 'install', 'status', 'doctor', 'uninstall'
    $hosts = 'claude', 'codex', 'gemini', 'all'

    $tokens = $commandAst.CommandElements | ForEach-Object { $_.ToString() }
    $verb = if ($tokens.Count -ge 2) { $tokens[1] } else { $null }

    $candidates =
        if ($tokens.Count -le 1 -or ($tokens.Count -eq 2 -and $wordToComplete -ne '')) { $verbs }
        elseif ($verb -eq 'plugin') {
            if ($tokens[-1] -eq '--host') { $hosts }
            elseif ($pluginSubs -contains $tokens[2] -and $tokens.Count -ge 3) { @('--host') }
            else { $pluginSubs }
        }
        elseif ($verb -eq 'list') { '--global', '--flat', '--all', '--tag', '--limit' }
        elseif ($verb -eq 'search') { @('--global') }
        elseif ($verb -eq 'adopt') { @('--undo') }
        elseif ($verb -eq 'project') { @('rename') }
        else { @() }

    $candidates |
        Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
}
