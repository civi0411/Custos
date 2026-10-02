@echo off
setlocal
set "ROOT_DIR=%~dp0"
set "CLI_SCRIPT=%ROOT_DIR%ui\cli\bin\custos-cli.js"

if "%1"=="--web" goto :run_web
if "%1"=="web" goto :run_web

:: Default: Run Custos CLI directly in current terminal
node "%CLI_SCRIPT%" %*
goto :eof

:run_web
echo [Custos] Starting Custos Web Interface on http://localhost:1420 ...
cd /d "%ROOT_DIR%ui\cli"
start "" "http://localhost:1420/"
npm run dev
goto :eof
