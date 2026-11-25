@echo off
setlocal
set "ROOT_DIR=%~dp0"

if "%1"=="--web" goto :run_desktop
if "%1"=="web" goto :run_desktop

:: Default: Run Custos native CLI directly in current terminal
cargo run -p custos-app-cli -- %*
goto :eof

:run_desktop
echo [Custos] Starting Custos Desktop UI on http://localhost:1420 ...
cd /d "%ROOT_DIR%apps\desktop\ui"
start "" "http://localhost:1420/"
npm run dev
goto :eof
