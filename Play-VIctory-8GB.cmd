@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem ---------------------------------------------------------------------------
rem Play-VIctory-8GB.cmd [4k|1440p|1080p|936p|720p|540p] [perf]: the settings that fit a
rem card that is short of memory, and a ladder of output sizes to find out
rem whether a frame is spent on pixels or on preparation.
rem
rem Written for the 8 GiB cards (RTX 4060/4070/5070 Laptop, 3060 Ti, 3070,
rem 5060), where the port runs out of a 7.5 GiB budget, spills into shared
rem memory, and then pays for it in paging: copy-backs that take a second and
rem a half for 35 MB, textures re-uploaded thousands of times, and a frame rate
rem in the single digits. On those cards the card itself measured under 12 ms
rem of work inside frames of 56 to 119 ms, so what is being paid for is memory,
rem not shading.
rem
rem THE LADDER IS THE POINT. Run it twice or three times at different sizes and
rem compare the median frame time:
rem   4K -> 1080p is 4 times fewer pixels, 1080p -> 540p another 4 times.
rem If the frame time falls by about those factors, the frame is spent on
rem pixels and the size is the lever. If it barely moves, the frame is spent on
rem preparing submissions, and no size will help: the project's own record says
rem the thread that prepares submissions took 48 ms of a 55 ms frame on an
rem RX 7900 XTX, and that preparing is the wall. A frame cannot be shorter than
rem the work of preparing it, so 30 fps needs preparing under 33 ms.
rem
rem Every switch below exists in this tree. The profile is the one the leading
rem continuation (cool_fox) ships for its "pareto-residency" launcher, minus
rem the two switches this tree does not carry (VICTORY_PORT_GUARD,
rem VICTORY_LIBRARY_STORE_LATER), plus the output size, which is the largest
rem single lever here and is off by default. Nothing in the engine changes;
rem set any VICTORY_* yourself to override a line (each is only a default).
rem ---------------------------------------------------------------------------

rem `Play-VIctory-8GB.cmd warm` is not a run. It makes the pipelines the title uses
rem ahead of time, from the book the runs write, so that the first world frame does not
rem compile them while you watch. That compile is the shape of the 300-600 ms frames:
rem the title wants over 1,200 new pipelines for its first world frame.
if /i "%~1"=="warm" goto warm

set "profile=%~1"
if not defined profile set "profile=1080p"
rem
rem Second argument: `perf` turns three preparing switches on that no one has seen
rem render on an 8 GiB card yet (see the block further down). Without it they are 0, so
rem a run with no second argument draws with the settings the leading continuation
rem ships. A run with no 3D is not a frame rate.
rem
rem Two switches and one more are NOT behind `perf` any more, because they were seen
rem to take the picture away on the 5070 8 GiB card (Logan-Gaillard, 2026-11-27):
rem   VICTORY_FILL_FORWARD / VICTORY_PLACED_ORDER  blanked a world frame (detail 0.32,
rem     luminance 20.3 against 91.6 on the reference build)
rem   VICTORY_TEXTURE_NO_WAIT=1  turns "waiting for room" into "absent", and the runs
rem     with it lost the environment and its textures (21.0 luminance, 45.5 % of the
rem     pixels under 16 against 0.3 % on the reference build)
rem They stay settable by hand, and their blocks below say why they are not defaults.
set "experiments=0"
if /i "%~2"=="perf" set "experiments=1"
rem
rem Third argument `60`: the title asks for 30 fps itself (two sites in its code, plus a
rem limiter of its own; crates/victory-loader/src/frame_rate.rs documents both), so a run
rem without this is capped at 33.3 ms whatever the hardware can do. This lifts the interval
rem to 16.7 ms and gives the title the matching rate. A measurement, not a fix: it makes
rem visible what the laptop can do, and it can tear.
if /i "%~3"=="60" (
    if not defined VICTORY_FRAME_INTERVAL_MS set "VICTORY_FRAME_INTERVAL_MS=16.667"
    if not defined VICTORY_TITLE_FPS set "VICTORY_TITLE_FPS=60"
)

if /i "%profile%"=="540p" set "lowres=1"
if /i "%profile%"=="540p" set "pixels=960x540"
if /i "%profile%"=="936p" set "pixels=1664x936"
if /i "%profile%"=="720p" set "pixels=1280x720"
if /i "%profile%"=="1080p" set "pixels=1920x1080"
if /i "%profile%"=="1440p" set "pixels=2560x1440"
if /i "%profile%"=="4k" set "pixels=3840x2160"
if not defined pixels (
  echo Unknown size "%profile%": use 540p, 720p, 936p, 1080p, 1440p or 4k. 1>&2
  exit /b 2
)

if not defined VICTORY_GAME_DIR (
  echo Set VICTORY_GAME_DIR to the folder containing your title executable. 1>&2
  exit /b 2
)
if not exist "%VICTORY_GAME_DIR%\game_scarlett_master.exe" (
  echo game_scarlett_master.exe not found in VICTORY_GAME_DIR: "%VICTORY_GAME_DIR%" 1>&2
  exit /b 2
)
set "runner=%~dp0target\release\victory.exe"
rem The frame CSV is behind the `diagnostics` feature (victory-gpu, host.rs:
rem `#[cfg(feature = "diagnostics")] mod frames;`), so a plain release build
rem writes no file and no line about it - which is what happened on the first
rem 8 GiB run: VICTORY_FRAMES was set and nothing appeared. A diag build is
rem release code generation with that feature on, at target\diag.
if defined VICTORY_FRAMES if not "%VICTORY_FRAMES%"=="0" (
  if exist "%~dp0target\diag\victory.exe" (
    set "runner=%~dp0target\diag\victory.exe"
  ) else (
    echo VIctory: VICTORY_FRAMES is set and there is no target\diag build: a release build writes no frame CSV.
    echo          cargo build --profile diag -p victory --features diagnostics
  )
)
if not exist "%runner%" (
  echo Build it first: cargo build --release --locked -p victory 1>&2
  exit /b 2
)

rem The pixel lever: at load the title is made to write this size into its own
rem settings instead of the 3840x2160 a Series X writes, and its screen size,
rem back buffers and render targets are all that size afterwards. 1080p is 4
rem times fewer pixels than 4K (8.29 MP to 2.07 MP) and 1.78 times fewer than
rem the 2560x1440 the title's own 2/3 upscaler would have rendered; 960x540 is
rem 16 times fewer than 4K. Under 4K the interface panels scale themselves
rem (VICTORY_UI_VIEWBOX, on by default).
rem
rem 540p is not one of the title's five modes, so it needs VICTORY_TEST_LOWRES,
rem which is the loader's own escape hatch for sizes below 720p: it writes the
rem size past the mode list (even dimensions, at least 256x144). Its comment
rem says "test runs only" because the title has no mode for it; on the sizes
rem above 720p that caveat does not apply at all.
rem
rem Each patch site is byte-compared first and left alone, with a line saying
rem so, if your build of the title differs: this cannot corrupt a run. If the
rem size did not take, the log's "presenting to a window" line still reports
rem 3840x2160 and drawn_of_the_screen stays at 2.025.
if defined lowres if not defined VICTORY_TEST_LOWRES set "VICTORY_TEST_LOWRES=1"
if not defined VICTORY_OUTPUT set "VICTORY_OUTPUT=%pixels%"
rem
rem 540p is the one rung the title may not take. On an 8 GiB laptop it asked
rem for 960x540 and the title presented 1280x720 (Logan-Gaillard, 2026-11-27),
rem so the size the window line reports is the size you got. 936p is one of
rem the title's own modes, so it cannot be clamped: 1.56 MP, between 720p
rem (0.92 MP) and 1080p (2.07 MP), and it is the low rung to trust.
if defined lowres (
  echo VIctory: 540p asks for 960x540 through VICTORY_TEST_LOWRES; the title may clamp it to 1280x720.
  echo          Read the log's "presenting to a window" line before comparing this run to another.
)

rem Card memory: mip levels take card memory only for the levels the title
rem holds, which the continuation measures at up to 3 GiB on this title, and
rem small buffers are kept at 64 KiB rather than 16 MiB apiece.
if not defined VICTORY_LEVELS_ON_DEMAND set "VICTORY_LEVELS_ON_DEMAND=1"
if not defined VICTORY_CARD_BUFFERS_MIN_KB set "VICTORY_CARD_BUFFERS_MIN_KB=64"

rem VICTORY_PIPELINE_WAIT_MS=0 is NOT set here, though an earlier version of
rem this file set it. On an 8 GiB laptop it dropped 5 643 draws over 728
rem pipelines ("draws dropped for pipelines not made yet") and the scene was
rem not drawn at all (Logan-Gaillard, 2026-11-27): the frame got fast because
rem its draws were gone. A frame that is fast because it was not drawn is not
rem a frame to measure. Set it yourself if you want the stall gone, and look at
rem the picture before you look at the clock.

rem Two preparing experiments, off in the engine, NOT measured by anyone on your
rem card, and off here too unless the second argument is `perf`: a run that does not
rem draw measures nothing, and these are the ones that can take the picture away.
rem
rem Two preparing experiments, off by default in the engine and NOT measured by
rem anyone on your card. Note what the engine already does for itself: on a
rem machine with 4 or more logical CPUs it spreads a flush's copies, the fills
rem of a submission and its textures over threads already (host.rs,
rem wide_refresh_enabled), so those are not knobs left unturned.
rem VICTORY_REFRESH_FRONT brings everything a submission
rem refreshes up to date in one go on several threads ahead of its steps,
rem instead of 580 copies one after the other on the submitting thread (3.4 to
rem 6.5 ms measured on one session); VICTORY_QUIET_WAITS leaves out a full wait
rem when nothing that writes was recorded since the last one. They are here
rem because preparing is the wall. If you see wrong textures, a black screen or
rem a device loss, run with VICTORY_REFRESH_FRONT=0 and VICTORY_QUIET_WAITS=0
rem set in your environment before launching, and say so: that is worth knowing.
if not defined VICTORY_REFRESH_FRONT set "VICTORY_REFRESH_FRONT=%experiments%"
if not defined VICTORY_QUIET_WAITS set "VICTORY_QUIET_WAITS=%experiments%"
rem
rem VICTORY_WIDE_MERGE is the one spreading switch the engine leaves off by
rem default: what shaders wrote, fetched and then merged one piece after
rem another on the thread that settles (about 630 pieces and 40 MB a frame),
rem taken instead by eight threads an eighth each.
if not defined VICTORY_WIDE_MERGE set "VICTORY_WIDE_MERGE=1"
rem
rem `perf` only (it moves waits, not what is drawn). The parts of one submission go to
rem the queue in one call instead of one
rem call a part (VICTORY_JOIN_ONE_CALL, off by default; Skyfrye's
rem AnyXboxX#337, with barriers standing in for the cuts). A frame that
rem hands the card 58 pieces, which is what the card-span columns measured on
rem an 8 GiB laptop, is 58 ExecuteCommandLists calls to pay for.
if not defined VICTORY_JOIN_ONE_CALL set "VICTORY_JOIN_ONE_CALL=%experiments%"
rem
rem `perf` only, for the same reason: the pair touches constants, which is the kind of
rem thing that blanks a frame when it is wrong.
rem
rem The pair that costs the least for what it gives: a copy at its place in a
rem list writes a few hundred bytes into the title's constants ring, and a
rem fill that reads it ahead of the copy made the whole submission wait for
rem the card to run everything - 13 to 28 ms a frame in runs trace and
rem trace2, 40 to 60 ms in run n19, which is the `stood_waiting_us` column.
rem VICTORY_FILL_FORWARD takes a waiting fill's bytes from the copy at its
rem place instead, and VICTORY_PLACED_ORDER keeps the fills ahead of the copy
rem on the old values, which is what the pair is documented for. Both are off
rem in the engine and this is the only place they are turned on. If constants
rem look wrong (camera, lighting, a smeared sky), set BOTH to 0 before
rem launching and say so: that is a real finding, not a user error.
rem 0 whatever the tier: this pair was measured blanking a world frame on the 5070
rem (one capture of run D, detail 0.32 against 2.68). A switch whose failure is
rem visible is the one allowed to be on, and this is that failure, seen.
if not defined VICTORY_FILL_FORWARD set "VICTORY_FILL_FORWARD=0"
if not defined VICTORY_PLACED_ORDER set "VICTORY_PLACED_ORDER=0"

rem Keeping the card's clocks up between submissions, and the submission
rem stages the continuation keeps on for this profile.
if not defined VICTORY_CARD_AWAKE set "VICTORY_CARD_AWAKE=1"
if not defined VICTORY_CARD_STEPS set "VICTORY_CARD_STEPS=2"
if not defined VICTORY_JOIN_COPIES set "VICTORY_JOIN_COPIES=1"
if not defined VICTORY_RELAY set "VICTORY_RELAY=2"
if not defined VICTORY_RELAY_COPY set "VICTORY_RELAY_COPY=1"
if not defined VICTORY_WIDE_BACK set "VICTORY_WIDE_BACK=1"
if not defined VICTORY_UPLOAD_AHEAD set "VICTORY_UPLOAD_AHEAD=1"
if not defined VICTORY_MAP_THREADS set "VICTORY_MAP_THREADS=1"
if not defined VICTORY_INDIRECT_WINDOW set "VICTORY_INDIRECT_WINDOW=1"
if not defined VICTORY_INDIRECT_BATCH set "VICTORY_INDIRECT_BATCH=2"
if not defined VICTORY_SHOW_AFTER set "VICTORY_SHOW_AFTER=1"
if not defined VICTORY_CARD_REHASH_RUNS set "VICTORY_CARD_REHASH_RUNS=1"
if not defined VICTORY_PORT_JOURNAL set "VICTORY_PORT_JOURNAL=1"

rem A texture that is waiting for room is not waited for.
rem Not a default and not behind `perf`: with it the world's environment and its
rem textures are absent rather than late (Logan-Gaillard, 2026-11-27: 21.0 luminance
rem against 91.6, 45.5 % of pixels under 16 against 0.3 %). Set it by hand to see a
rem run without the waits; do not read a frame time from that picture.
if not defined VICTORY_TEXTURE_NO_WAIT set "VICTORY_TEXTURE_NO_WAIT=0"

rem A constant, not the frame estimate: walking into a bright doorway raised
rem the estimate and washed the whole frame out. constant:1.0 is even light.
if not defined VICTORY_AMBIENT set "VICTORY_AMBIENT=constant:0.8"
if not defined VICTORY_AMBIENT_AIR set "VICTORY_AMBIENT_AIR=1.5"

rem The write guard misses some of the title's writes (skin on a guard's arms,
rem the phone's frame read garbage until every check hashed the texture again).
rem Costs CPU; the frame rate held at 5-7 fps with it on.
if not defined VICTORY_TEXTURE_GUARD set "VICTORY_TEXTURE_GUARD=0"

rem The asynchronous submission paths stay off on NVIDIA: with THIN the head
rem skin of people in play is not drawn (eyes and teeth are), and JOIN_PARTS
rem repeats frames of the opening video. Set them to 1 to try instead.
if not defined VICTORY_THIN set "VICTORY_THIN=0"
if not defined VICTORY_JOIN_PARTS set "VICTORY_JOIN_PARTS=0"

rem The book of the pipelines the title uses, so that `Play-VIctory-8GB.cmd warm` can
rem make them ahead of a run. A small file, a write per new pipeline, and the only way the
rem first world frame stops being the longest one of the session.
if not defined VICTORY_PIPELINE_BOOK set "VICTORY_PIPELINE_BOOK=1"

if not defined VICTORY_CACHE_DIR set "VICTORY_CACHE_DIR=%LOCALAPPDATA%\VIctory\cache-8gb"
if not defined VICTORY_LOG if not defined RUST_LOG set "VICTORY_LOG=warn"

echo VIctory: %profile% [VICTORY_OUTPUT=%VICTORY_OUTPUT%] experiments=%experiments% runner=%runner%
if not "%experiments%"=="1" echo VIctory: three speculative switches are off (no 3D last time? this is the run that draws).
if "%experiments%"=="1" echo VIctory: three switches ON that have never been seen to render on an 8 GiB card: if the picture is wrong, drop "perf".
if defined VICTORY_FRAMES echo VIctory: frames to %VICTORY_FRAMES%
if defined VICTORY_FRAME_INTERVAL_MS echo VIctory: 30 fps lifted to %VICTORY_FRAME_INTERVAL_MS% ms (a measurement, not a fix).
"%runner%" run "%VICTORY_GAME_DIR%\game_scarlett_master.exe" --console series-x --execute

goto :eof

:warm
rem The release build plays, so it warms; the diag build warms the same book.
set "runner=%~dp0target\release\victory.exe"
if not exist "%runner%" set "runner=%~dp0target\diag\victory.exe"
if not exist "%runner%" (
    echo VIctory: no victory.exe built yet: cargo build --release -p victory
    exit /b 1
)
if not defined VICTORY_CACHE_DIR set "VICTORY_CACHE_DIR=%LOCALAPPDATA%\VIctory\cache-8gb"
echo VIctory: warming the pipelines the book names, with %runner%
"%runner%" warm

exit /b %errorlevel%
