[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ScriptArgs
)

if ($ScriptArgs -contains "--web" -or $ScriptArgs -contains "web") {
    Write-Host "[Custos] Starting Custos Desktop UI on http://localhost:1420 ..." -ForegroundColor Cyan
    Set-Location -Path (Join-Path $PSScriptRoot "apps\desktop\ui")
    Start-Process "http://localhost:1420/"
    npm run dev
} else {
    $cliScript = Join-Path $PSScriptRoot "packages\custos-npm-package\repl\custos-cli.js"
    & node $cliScript @ScriptArgs
}
