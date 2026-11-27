@echo off
setlocal enabledelayedexpansion
rem ===========================================================================
rem  VIctory on an 8 GiB laptop: double-click this file. No arguments.
rem
rem  It finds the layer and your copy of the title, records the run to a CSV,
rem  makes the pipelines ahead of time, and starts the game at 1080p with the
rem  settings that draw. When it ends it tells you which file to send back.
rem
rem  Nothing of the game is in this folder except your own dump. The layer
rem  writes only under %LOCALAPPDATA%\VIctory.
rem ===========================================================================

set "here=%~dp0"

rem --- the layer ------------------------------------------------------------
set "runner=%here%target\diag\victory.exe"
if not exist "%runner%" set "runner=%here%target\release\victory.exe"
if not exist "%runner%" if exist "%here%victory.exe" set "runner=%here%victory.exe"
if not exist "%runner%" (
  echo.
  echo   No victory.exe here.
  echo.
  echo   Either download victory-diag.exe from the Releases page and put it in
  echo   this folder under the name victory.exe, or build it:
  echo.
  echo     cargo build --profile diag -p victory --features diagnostics
  echo.
  pause
  exit /b 1
)

rem --- your copy of the title ----------------------------------------------
if not defined VICTORY_GAME_DIR if exist "%here%game_scarlett_master.exe" set "VICTORY_GAME_DIR=%here%"
if not defined VICTORY_GAME_DIR if exist "%here%dump\game_scarlett_master.exe" set "VICTORY_GAME_DIR=%here%dump"
if not defined VICTORY_GAME_DIR (
  echo.
  echo   Where is your copy of the title? The folder that holds
  echo   game_scarlett_master.exe. Drag it onto this window, then press Enter.
  echo.
  set /p "VICTORY_GAME_DIR=  Folder: "
  for /f "delims=" %%i in ("!VICTORY_GAME_DIR!") do set "VICTORY_GAME_DIR=%%~i"
)
if not exist "!VICTORY_GAME_DIR!\game_scarlett_master.exe" (
  echo.
  echo   game_scarlett_master.exe is not in "!VICTORY_GAME_DIR!".
  pause
  exit /b 2
)

rem --- the run, recorded by default ---------------------------------------
rem The frame CSV needs the diagnostics build; the release build has no
rem recorder at all and would say nothing. A name that does not depend on the
rem machine's date format, so it cannot fail on a locale.
if not defined VICTORY_FRAMES set "VICTORY_FRAMES=%LOCALAPPDATA%\VIctory\runs\run-%RANDOM%%RANDOM%.csv"
if not exist "%LOCALAPPDATA%\VIctory\runs" mkdir "%LOCALAPPDATA%\VIctory\runs"
set "VICTORY_CARD_SPANS=1"

echo.
echo   Recording to  %VICTORY_FRAMES%
echo   Title folder  !VICTORY_GAME_DIR!
echo.

rem --- pipelines ahead of time --------------------------------------------
echo   Making the pipelines the last runs wrote down, ahead of time.
echo   The first time after a new build this takes minutes; it is what keeps
echo   the first world frame from being the longest of the session.
call "%here%Play-VIctory-8GB.cmd" warm

rem --- play ---------------------------------------------------------------
call "%here%Play-VIctory-8GB.cmd" 1080p
set "code=%errorlevel%"

echo.
echo   The run is over ^(exit %code%^).
echo.
echo   Send back, both:
echo     1. the file  %VICTORY_FRAMES%
echo     2. a screenshot of the world itself - not a menu - so the picture can
echo        be checked as well as the numbers.
echo.
echo   If the picture was wrong, run Play-VIctory-8GB.cmd 1080p again with
echo   VICTORY_FILL_FORWARD=0 VICTORY_PLACED_ORDER=0 VICTORY_TEXTURE_NO_WAIT=0
echo   already set - they are off by default, and a wrong picture with them off
echo   is a finding, not a mistake on your side.
echo.
pause
exit /b %code%
