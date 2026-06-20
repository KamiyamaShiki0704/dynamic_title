@echo off
setlocal

if "%~1"=="" (
  echo Drag one or more .bk2 files onto this BAT, or run:
  echo   BK2ToMP4.bat file1.bk2 file2.bk2
  pause
  exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0BK2ToMP4.ps1" %*
echo.
pause
