@echo off
rem DOCL 一键启动开发模式（双击运行）
title DOCL dev
cd /d "%~dp0"
echo Starting DOCL dev...
npm run tauri dev
pause
