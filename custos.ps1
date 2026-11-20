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
    cargo run -p custos-app-cli -- @ScriptArgs
}
