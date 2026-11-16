@echo off
setlocal
set "ROOT_DIR=%~dp0"
set "CLI_SCRIPT=%ROOT_DIR%packages\custos-npm-package\repl\custos-cli.js"

if "%1"=="--web" goto :run_desktop
if "%1"=="web" goto :run_desktop

:: Default: Run Custos CLI directly in current terminal
node "%CLI_SCRIPT%" %*
goto :eof

:run_desktop
echo [Custos] Starting Custos Desktop UI on http://localhost:1420 ...
cd /d "%ROOT_DIR%apps\desktop\ui"
start "" "http://localhost:1420/"
npm run dev
goto :eof
