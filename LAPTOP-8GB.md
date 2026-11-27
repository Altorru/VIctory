# VIctory for an 8 GiB laptop

A derived tree of [101Leafy/victory-leafy](https://github.com/101Leafy/victory-leafy)
(GPL-3.0, which is itself Marvisak's [VIctory](https://github.com/Marvisak/VIctory) with
other people's work on top). This one has a single purpose, and it is written down in
[`docs/thirty-fps.md`](docs/thirty-fps.md): **thirty frames a second, sustained, on a
laptop card with 8 GiB of its own memory** — the machine that exists here is an
**RTX 5070 Laptop, 8 GiB, 2560x1600, Windows 11**.

Everything in this repository is either a measurement someone took, or a switch whose
failure is visible in the picture. Nothing is a claim that has not been run.

## Where it stands, in numbers (2026-11-27)

Four builds, one dump, 140 s each, killed by `timeout`, screenshots from the game window.
`A` is the reference; `B` is this branch with nothing set.

| | A — reference | B — this branch, no switch | C — + `1080p` | D — + `1080p perf` |
|---|---|---|---|---|
| outcome | 1,451 frames | **crash** (~85 frames) | 1,343 frames | 1,316 frames |
| `frame_us` median | 33.4 ms | — | 33.3 ms | 33.4 ms |
| `card_idle_us` median | 29.9 ms | — | 32.5 ms | 32.6 ms |
| `vram_used` peak | 7.95 GB | 1.57 GB | **9.48 GB** | 9.25 GB |
| budget | 7.47 GB | | 7.47 GB | 7.47 GB |
| the picture | luminance 91.6, 0.3 % of pixels under 16 | — | 21.0, **45.5 %** under 16 | one capture flat (detail 0.32) |

Three readings, and they are not comfortable ones:

1. **33.4 ms is the title's own frame interval, not this layer's limit.** The title asks for
   30 fps itself (`crates/victory-loader/src/frame_rate.rs` documents the two sites and its
   own limiter). At 1080p the reference build is therefore already at 30 in the median with
   the card idle 29.9 of those 33.4 ms. What costs the average — 1,451 frames in 140 s is
   10.4 a second — is a long tail of 300–600 ms frames.
2. **The card is 2.2 GB past its budget and the world's textures are missing.** `45.5 %` of
   pixels under 16 is not a slow world, it is an absent one.
3. **The crash in B is not attributed to anything yet.** A is 1,262 commits behind this
   branch's base, so A against B differs by this branch *and* by a day of upstream work. The
   control is `rebase/logan-adapter` — head plus the adapter and nothing else.

## What is in this branch

- **`Play-VIctory-8GB.cmd`** — the launcher for an 8 GiB card: a size ladder
  (`4k` / `1440p` / `1080p` / `936p` / `720p` / `540p`, and the title clamps `540p` to 720p),
  and two tiers. `Play-VIctory-8GB.cmd 1080p` sets only what draws;
  `Play-VIctory-8GB.cmd 1080p perf` adds the three switches that move waits without moving
  what is drawn. Every switch it sets is a line you can read and delete.
- **The frame CSV** (`VICTORY_FRAMES`) — one row a frame: time, preparing, copies, textures,
  draws, the card's own clock, memory. It needs the diagnostics build:
  `cargo build --profile diag -p victory --features diagnostics`.
- **`snapshot_bytes`** — what two submissions in flight would cost in memory. It answered:
  690 MB median, so the two-in-flight step is not payable on 8 GiB, and it is out of the plan.
- **Three memory gauges** — `ours_host_textures`, `ours_targets_reserve`, `ours_card_room`:
  this port's own share of `vram_used`, so an overrun can be attributed instead of guessed.
- **The card's own account of itself** — `card_span_*`: submissions a frame (in hundredths),
  the card working, the card idle between submissions, the longest one.
- **`Play-VIctory-8GB.cmd warm`** — not a run: it makes the pipelines the title uses ahead
  of time from the book the runs write (`VICTORY_PIPELINE_BOOK=1`, set by the launcher),
  so the first world frame stops compiling more than 1,200 pipelines while you watch. That
  compile is the shape of the 300–600 ms frames.
- **A third argument, `60`** — `Play-VIctory-8GB.cmd 1080p perf 60` lifts the title's own
  30 fps interval to 16.7 ms. The title asks for 30 itself, so without this a run is capped
  at 33.3 ms whatever the laptop can do; this is the measurement that shows the headroom,
  not a fix (it can tear).
- **`VICTORY_WATER_LIGHT=0`** — leaves out the two tiled water-light programs, which on a
  32-lane host cost about 15 ms of the card a frame and leave dark blocks on open water.
  Ported from VIctory-NVIDIA, where it was measured on an RTX 5070 (25 → 30 fps); off by
  default here because it changes the picture: the water comes out darker and bluer.

## Build and run

```
cargo build --profile diag -p victory --features diagnostics
```

Then, next to your own dump of the title (v1.0.2.1, the folder with
`game_scarlett_master.exe`):

```
set VICTORY_FRAMES=D:\runs\a-1080p.csv
set VICTORY_CARD_SPANS=1
Play-VIctory-8GB.cmd 1080p
```

`diag` is slower than `--release` and it is the only build that writes the CSV; the launcher
picks `target\diag\victory.exe` itself when `VICTORY_FRAMES` is set. With `--release` the
recorder does not exist and the file is not written — that is not a path problem.

## Continuous integration

`.github/workflows/laptop-8gb.yml`, on every push to `laptop-8gb` and every pull request:

| job | what it proves |
|---|---|
| `check` | the workspace still compiles on Windows, all targets, and with `--features diagnostics` |
| `test` | `victory-gpu`'s own unit tests: the reserve a small card keeps, the column names of the CSV, counters that go back to zero |
| `lint` | clippy, reported but **not fatal** — two warnings predate this branch (a `is_multiple_of()` in victory-formats, a collapsible `if` in victory-gpu's build script) and turning them into a red badge would hide the signal people need |
| `diag-build` | the diagnostics binary exists at `target/diag/victory.exe`, where the launcher looks, and is uploaded as an artifact |

`.github/workflows/release-8gb.yml`, on a `v*` tag: both binaries, `SHA256SUMS.txt`, the
launcher and the plan, attached to the release. `victory-diag-*` is the one that reports;
`victory-release-*` is the one that plays.

## What is not here

- No game files, no dumps, no shader dumps, no assets, nothing of NVIDIA's. Build the layer;
  supply your own copy of the title.
- No texture or model of any kind, and no measurements invented: every number above came
  from a file that was kept, and the ones that came from arithmetic have been corrected in
  public eight times, three of them in this very document's subject matter.

## Licence and credit

GPL-3.0-or-later, as the trees it stands on. The history is kept, and the commits that come
from elsewhere name their source. This tree is derived from
[101Leafy/victory-leafy](https://github.com/101Leafy/victory-leafy) at `full-build`; the
plan, the launcher and the CSV columns are this branch's own work, and everything else is
someone else's.
