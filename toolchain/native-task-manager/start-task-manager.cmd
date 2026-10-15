@echo off
setlocal DisableDelayedExpansion
title Sovra Task Manager
if not defined LOCALAPPDATA (
    echo Cannot locate your Windows local application data folder.
    pause
    exit /b 1
)
set "SOVRA_TASK_DATA=%LOCALAPPDATA%\Sovra\Task Manager"
if not exist "%SOVRA_TASK_DATA%\" mkdir "%SOVRA_TASK_DATA%"
if not exist "%SOVRA_TASK_DATA%\" (
    echo Cannot create the Task Manager data folder.
    pause
    exit /b 1
)
"%~dp0task-manager.exe" interactive "%SOVRA_TASK_DATA%\tasks.txt"
set "SOVRA_TASK_EXIT=%ERRORLEVEL%"
if not "%SOVRA_TASK_EXIT%"=="0" pause
exit /b %SOVRA_TASK_EXIT%
