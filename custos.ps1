[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ScriptArgs
)

if ($ScriptArgs -contains "--web" -or $ScriptArgs -contains "web") {
    Write-Host "[Custos] Starting Custos Web Interface on http://localhost:1420 ..." -ForegroundColor Cyan
    Set-Location -Path (Join-Path $PSScriptRoot "ui\cli")
    Start-Process "http://localhost:1420/"
    npm run dev
} else {
    $cliScript = Join-Path $PSScriptRoot "ui\cli\bin\custos-cli.js"
    & node $cliScript @ScriptArgs
}
