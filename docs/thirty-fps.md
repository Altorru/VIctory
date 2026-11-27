# Thirty frames a second, and the arithmetic it has to obey

This is the plan, in order, with what each step removes and what would prove it did
not work. It is written against three sets of numbers, none of them mine to invent:

| Where | Frame | Preparing a submission | The card |
|---|---|---|---|
| RX 7900 XTX, 1440p (`findings.md`) | 46-65 ms | **48 ms** | 41 ms busy |
| RTX 5070 Laptop 8 GiB, 4K, world load (Logan, 2026-11-27) | 629 ms median over the frames over 300 ms | **93 ms** | 378 ms busy, 111 ms idle between submissions, longest 234 ms |
| RTX 5060 Ti, in play (cool_fox's report) | 56-119 ms | 38-92 ms | **under 12 ms** |

The last row matters as much as the second: on a slower card than Logan's the card
is *not* the wall in play, and 12 ms of card work fits inside a 33 ms frame with
room to spare. His 378 ms is a world load, where everything is being made for the
first time. Whether the card is a wall in play on his machine is one of the things
the test run is for — and it is a question, not a conclusion.

## What the four builds said (Logan-Gaillard, RTX 5070 Laptop 8 GiB, 2026-11-27 13:35)

One dump (`b99f876b74d4a77f…`), same cache, 140 s a run killed by `timeout`, screenshots
taken from the game window. Four builds:

| | A — reference | B — everything, no switch | C — everything + `1080p` | D — everything + `1080p perf` |
|---|---|---|---|---|
| build | `20eb157` + the adapter | `bf0e145`, diag | same | same |
| outcome | 1,451 frames, no crash | **crash** (`0x80000003`, ~85 frames) | 1,343 frames | 1,316 frames |
| `frame_us` median | 33.4 ms | — | 33.3 ms | 33.4 ms |
| `card_idle_us` median | 29.9 ms | — | 32.5 ms | 32.6 ms |
| `vram_used` peak | 7.95 GB | 1.57 GB at death | **9.48 GB** | 9.25 GB |
| `vram_shared` peak | 6.82 GB | — | **9.64 GB** | 9.16 GB |
| budget | 7.47 GB | | 7.47 GB | 7.47 GB |
| `drawn` median | 10 (p90 22) | — | 10 (p90 22) | 10 (p90 22) |
| the picture | luminance 91.6, 0.3 % of pixels under 16, detail 2.68 | — | 21.0, **45.5 %** under 16 | one capture at detail **0.32** (a flat field) |

Four things, in the order they change the plan.

**1. A is not the control it looks like, and the comparison is confounded.** A is
`20eb157`, a `full-build` commit from 2026-11-26 13:50. `full-build` is now `bccee2c`,
and there are **1,262 commits between them** (`git log --oneline 20eb157..origin/full-build
| wc -l`). So A against B differs by my pull requests *and* by a day of upstream work.
The control for "is it mine" is `rebase/logan-adapter` — that is head plus the adapter and
nothing else, two commits on top of `bccee2c`. Until that build runs on this dump, the
crash is not attributed to anything, and I should not have let a day-old base be called the
reference in the first place.

**2. The median frame at 1080p is the title's own clock, not ours.** 33.4 ms with the card
idle 29.9 of it, in A, C and D alike: all three sit at ~30 Hz with the card nearly idle.
That revises what I said last cycle and it has to be said plainly: **93 ms of preparing is a
4K world-load figure, not a 1080p one.** "A frame cannot be shorter than its preparing, so
the ceiling is 10.8 fps at any size" is wrong as stated — it is a ceiling for *that kind of
frame*, and at 1080p the reference build already sits at 33 ms in the median. What is still
true: 1,451 frames in 140 s is 10.4 a second on average against a 33.4 ms median, so the
average is eaten by a long tail of stalls, which is a different problem from the one the
plan was aimed at.

**3. `budget_refused` was a bad name and misled the reader it was written for.** It is
`BUDGET_NOTES[0]`: the **indirect records that ran empty for want of their command list's
2^27 budget** inside the shaders (`host.rs:8167`, reported by the frame-rate line as
"indirect records a frame ran empty"). It has nothing to do with card memory, and a run with
it at 0 says nothing about whether the port refused an upload. The column is `records_refused`
now, with that in a comment. So "9.6 GB against a 7.47 GB budget with `budget_refused` 0" is
not a contradiction — it is two unrelated facts, and the memory question is still open.

**4. The new gauges are what closes it without another bisect.** `vram_used` alone cannot
say whether the 2.2 GB past the budget is ours or the title's and the driver's, and no
switch of ours can refuse theirs. The CSV now carries `ours_host_textures`,
`ours_targets_reserve` and `ours_card_room` beside it: the host copies of the title's
textures, the room kept back for the world's render targets (#26's figure), and the room
kept for card copies of buffers not made yet.

Two items the same run settles:

- **Step 3 is not payable, and the column says so.** `snapshot_bytes` is **690 MB median**
  per submission stream (108 of 1,343 frames carry a non-zero sample), 699 MB peak. Two
  submissions in flight is ~1.4 GB of snapshots on a card already 2.2 GB past its budget.
  Step 3 is closed here, by measurement, rather than by argument.
- **The fill-forward pair is off everywhere now.** One capture of run D is a flat field
  (detail 0.32 against 2.68): `VICTORY_FILL_FORWARD` + `VICTORY_PLACED_ORDER` blanked a world
  frame. That is the failure mode the pair was allowed to have *because* it would be seen;
  it was seen, so it is out of the launcher's tiers. Its 13-28 ms (and 40-60 ms in `n19`)
  were real waits removed, and the picture was the price.

`VICTORY_TEXTURE_NO_WAIT=1` is out of the launcher's default tier for the same kind of
reason, and it is the likelier cause of the missing environment: it turns "waiting for room"
into "absent", and the run with it had 45.5 % of its pixels under 16 while the reference had
0.3 %. The 21.0 luminance is a world with its textures gone, not a world drawn slowly.

## The arithmetic, before any switch

A frame cannot be shorter than the work of preparing it: the card is handed the
frame when the preparing is done. So, with `P` the preparing and `C` the card:

    frame >= max(P, C)        and, on this port, usually frame = P + something

30 fps is 33 ms. Two conditions, both necessary:

1. **P <= 33 ms.** Today on Logan's machine P is 93 ms. That is 2.8 times too much.
2. **C <= 33 ms at the size chosen.** Plausibly already true at 1080p in play (12 ms
   on a 5060 Ti), almost certainly not during a world load.

And one number that names the lever inside P:

    93 ms / 58 submissions a frame = 1.6 ms a submission

58 is measured (`card_span_submissions_x100`, in hundredths: 5 800 is 58). So:

- **any 20 submissions a frame would be about 32 ms of preparing**, if each costs
  what it costs today;
- **or 0.57 ms each**, if the count stays at 58.

Thirty fps is one of those two, or a mixture. There is no switch that is both, which
is why this is a plan and not a line in a launcher.

## Where the 1.6 ms a submission goes

Proportions from the XTX's 48 ms (`findings.md`, "Where a frame goes"), which are the
only breakdown anyone has measured, and which agree with the split of Logan's 93 ms
(`copy_us` 30 ms = 32 %, the rest 63 ms = 68 %):

| Part | Of 48 ms | Of Logan's 93 ms, if it scales | What it is |
|---|---|---|---|
| Steps of the lists | 18 | ~35 | recording what the card will do |
| — refreshes of host copies | 10 | ~19 | noticing and moving title writes |
| Steps done on the card | 7 | ~14 | barriers, copies, queries |
| Waiting for descriptor records | 6 | ~12 | the title's tables, mirrored |
| Asking the write watch | 5 | ~10 | guards asked what changed |
| Copies in the card's memory | 4 | ~8 | keeping the working set resident |
| Title's bytes under shader writes | 4 | ~8 | comparing what the card wrote |

Under 33 ms means removing about 60 ms from that column. The rows are not
interchangeable and do not add up to a fifth of what they look like separately, but
they say where to cut first: **the steps of the lists**, at a third of it, and the
**refreshes** inside them.

## The path, in the order I would take it

Each step says what it should remove, what measures it, and what would falsify it.
Nothing here is claimed to work. Only the first step is written; the rest is work.

### 0. What is already in the pull requests, and costs nothing to try

- The device on the high-performance adapter (**#27**, Logan's) — without it a hybrid
  laptop runs on the integrated GPU, which is where his first run's one frame a second
  came from. This is a fault, not an optimisation.
- Card memory following the budget, render targets let go (**#26**, **#29**) — 6.44 GiB
  of *shared* memory out of 7.08 used is memory the card fetches over PCIe.
- The gauges off the frame's clock, and the card's own account of itself (**#24**).
- The launcher, thirty switches (**#31**), the integration of all of them (**#32**).

### 1. Fewer pieces: `VICTORY_JOIN_ONE_CALL=1` (in the launcher already)

The parts of a submission go to the queue in one call instead of one call a part,
with barriers standing in for the cuts (Skyfrye, AnyXboxX#337).

- Removes: a `ExecuteCommandLists` per part, and the driver's work behind it. With 58
  submissions this is 58 calls a frame.
- Does **not** claim: to remove preparing. It is in `stood_waiting`, not `preparing_us`.
  Saying otherwise would be the fifth number I have had to correct publicly.
- Measures it: `stood_waiting_us` and `card_span_between_us` against `card_span_busy_us`.
- Falsified if: `preparing_us` does not move and neither does the frame. Then this is a
  wait that was not there, and the next step is not a call to the queue.

### 2. Split one submission over two threads (the first item of "what is next")

What one submission does, done over two threads in a row. `findings.md` measured
preparing on a thread of its own as 103 → 75 ms, so threads do pay here; this is the
same idea one level down.

- Should remove: about half of the 68 % that is steps (≈ 30 ms of 93).
- Costs: memory. Snapshots and the ranges shaders write are **per host copy, not per
  submission** (`Host::submit` holds `self.submitted` from preparation to copy-back for
  exactly this reason), so two threads in flight means two sets of that. On an 8 GiB
  card with 6.44 GiB already shared, this is the step that can lose more than it gains.
- Measures it: `preparing_us` median, with `vram_shared` and `card_fetched_bytes` beside
  it — if shared goes over the budget, the gain is a loan.
- Falsified if: `preparing_us` falls but `frame_us` does not (the work moved off the
  critical path without shortening it), or shared memory eats the difference.

### 3. Ask for the next submission while this one is sent (the second item)

**Closed by measurement, 2026-11-27.** `snapshot_bytes` — the ranges shaders write, copied
per submission — is 690 MB median and 699 MB peak on the 8 GiB machine. Two in flight is
~1.4 GB, on a card already 2.2 GB past its budget with the world's textures missing. The
step is not payable in memory as the branch stands, and it is no longer in the plan until
the overrun in §"What the four builds said" has a cause.

Today the preparing thread waits on the submission it just handed over, because
`self.submitted` is held from preparation to copy-back.

- Should remove: the serialisation of preparing behind the send, which is the part of
  `stood_waiting_us` (212 ms on Logan) that is ours and not the card's.
- Costs: the same memory as step 2, for the same reason. **So steps 2 and 3 are one
  decision, not two**, and the cheapest first is to measure the memory (step 0's
  `vram_shared`) before writing either.
- Falsified if: `stood_waiting_us` falls but `preparing_us` and `frame_us` do not.

### 4. Narrow what the steps touch

The record of what has been tried: descriptor records of draws' tables (280 → 124 ms),
guards looked at first (69 → 6 ms of threads' time), AVX2 over the title's bytes under
shader writes (14 → 4-7). Narrowing has paid more than any single rewrite here.

- Should remove: part of the 6 ms of descriptor records and the 5 ms of write watch
  (≈ 12 and 10 ms of Logan's 93).
- Measures it: `preparing_us` with `copy_bytes` flat — narrowing must not move bytes.
- Falsified if: bytes move too, which would mean it was a width change, not a narrowing.

### 5. Stop making host copies the card already has

"View the card's own buffers where fills copy them now" (item 1 of "what is next").

- Should remove: the 10 ms of 48 that is refreshes, ≈ 19 of Logan's 93.
- Measures it: `copy_us` and `copy_bytes` inside `preparing_us` — this is the one step
  that must move bytes down, and the only one where a fall in bytes is the proof.
- Falsified if: `copy_bytes` falls and `preparing_us` does not. Then moving memory was
  never the cost, and the copies were paying for something else.

### 6. Then the size, and only then

If P is under 33 ms and C is not: the ladder's own arithmetic, in pixels.

| size | MP | fewer pixels than 1080p | card, from 378 ms at 4K | card, from 12 ms in play at 1080p |
|---|---|---|---|---|
| 4k (3840x2160) | 8.29 | 4x more | 378 ms | 48 ms |
| 1080p (1920x1080) | 2.07 | 1 | 94 ms | 12 ms |
| 936p (1664x936) | 1.56 | **1.33** | 71 ms | 9 ms |
| 720p (1280x720) | 0.92 | **2.25** | 42 ms | 5 ms |
| 540p (960x540) | 0.52 | **4** | 24 ms | 3 ms |

So **936p buys a quarter of the frame back, not half**, and 720p — the smallest size the
title actually presents on the 8 GiB laptop — is still 42 ms from that world load, over 33.
540p would be the only rung under 33 ms in a load, and it is the one the loader asks for and
the title clamps: `VICTORY_TEST_LOWRES=1` + `VICTORY_OUTPUT=960x540` presented
**1280x720** (Logan-Gaillard, 2026-11-27). The launcher now says so before the run rather
than after it: read the `presenting to a window` line before comparing two runs.

## Ported from VIctory-NVIDIA (2026-11-27)

Their tree and this one share their ancestor (`ffe310eb`, 2026-11-24): 1,193 commits on
their side, 1,230 here. What came over, and what it is worth:

- **The water's tiled light left out** (`pipeline/make.rs`, `VICTORY_WATER_LIGHT=0`): two
  programs (`5dc3b25986622949`, `6f225b641c5b0708`) that a 32-lane host only gets in the
  wave-wide form, where they leave whole tiles of open water dark and cost **about 15 ms of
  the card a frame** — their measurement on an RTX 5070: 25 fps with them, 30 without. Their
  tree leaves them out by default; here they are kept unless `VICTORY_WATER_LIGHT=0`, because
  the water comes out darker and bluer, and a different picture is not a free win.
- **The pipeline warm-up** (`Play-VIctory-8GB.cmd warm`, `VICTORY_PIPELINE_BOOK=1`): the
  mechanism was already in this tree; the launcher now uses it, because the compile of
  >1,200 pipelines at the first world frame is what the 300-600 ms frames are made of.
- **The title's 30 fps interval, lifted** (`Play-VIctory-8GB.cmd 1080p perf 60`): also
  already here (`victory-loader/src/frame_rate.rs`), now reachable without knowing the
  switch. It is the only way to see what the laptop can do.

## The levers already in the tree, and why only one is switched on

Audited 2026-11-27: every switch that is **off by default** in `victory-gpu`, read from
the comment where it is defined rather than guessed at. The sizes are the ones those
comments measured.

| Switch | What it removes | What it risks | Out |
|---|---|---|---|
| `VICTORY_FILL_FORWARD=1` with `VICTORY_PLACED_ORDER=1` | a submission standing **13-28 ms** (runs trace, trace2) and **40-60 ms** (n19) for a fill that reads the constants ring behind a copy at its place — this is `stood_waiting_us` | a fill ahead of a copy keeps the old values: camera, lighting, a smeared sky | both to 0 |
| `VICTORY_GUARD_FALLBACK_LATER=1` | ~210 `VirtualProtect` calls a frame made **under the lock**, 11.5 µs each (a5c442eec) | the guard's work with the lock let go: a write could be missed between the two | 0 |
| `VICTORY_MAP_REST=1` | ~1 967 blocks made read-only again a frame, most opened by a mapping just before (a5c442eec) | a write into a mapping-opened block noticed later than now | 0 |
| `VICTORY_TEXBACK_SAME=1` | a landing puts only the bytes that differ (19 a frame, 5 MB, the boat window) | a compare in place of a write; small | 0 |
| `VICTORY_BAKE_SYNC=1` | a bake draw waiting for its uploads on the lists | needs lists recorded on one thread (the launcher sets `VICTORY_THIN=0`) | 0 |
| `VICTORY_HOST_QUERIES=1` | occlusion queries kept on the host heap | culls draws; its own comment says "until a run shows that measured occlusion culls nothing our dropped draws should have drawn" | 0 |
| `VICTORY_PORT_CONSTANTS=1` | renderer port #1: the title's memory used in place | the largest change of the set, unmeasured on this card | 0 |

**The first row is the only one `Play-VIctory-8GB.cmd` sets, and only when the second
argument is `perf`.** The other four speculative switches the launcher used to set — the
two preparing experiments, the one-call submission, and this pair — are off by default
now, because the last 8 GiB run rendered no 3D with all of them on and **a run with no
3D is not a frame rate**. `Play-VIctory-8GB.cmd 1080p` draws with what the leading
continuation ships; `Play-VIctory-8GB.cmd 1080p perf` turns on the three that only move
waits (`REFRESH_FRONT`, `QUIET_WAITS`, `JOIN_ONE_CALL`), while this pair and
`VICTORY_TEXTURE_NO_WAIT` stay 0 in both tiers: they were measured taking the picture away
on the 5070 (`docs/thirty-fps.md`, the four builds), and the point of
the pair is that its failure is visible in the picture rather than silent in a lost write:
constants that read old values look wrong, and a person watching says so. The rest are not set, and the
reason is the same for all of them: their failure mode is *a write nobody noticed*, the
class this repository's own findings record as **six in two hundred runs**. A default is
not the place for a defect that needs two hundred runs to appear.

And the distinction that keeps this honest: **all of these shorten waits, not
preparing.** `stood_waiting_us` is 212 ms on an 8 GiB laptop and `preparing_us` is 93 ms;
the first row may take tens of milliseconds out of the first and nothing out of the
second. Frames that are lost waiting can come back this way. Frames that are lost
preparing cannot.

## What would make me say thirty is out of reach

Stated in advance, so it is not a conclusion moved to fit the result:

- **`preparing_us` does not move when submissions fall from 58 to 20.** Then the cost
  is not per submission, and the arithmetic above is the wrong model.
- **Preparing falls but `frame_us` does not**, in both step 2 and step 3. Then the
  critical path is elsewhere and this plan is aimed at the wrong wall.
- **The card is over 33 ms in play at 1080p on an 8 GiB card.** Then the size is the
  only lever left, and the answer is 540p frames, not 30 fps at 1080p.
- **Shared memory above the budget after step 2.** Then the split cannot be bought at
  this memory, and 30 fps waits for a card with more of it.

## What the code says about each step (read 2026-11-27, before writing any of it)

Going through the steps one by one, against the code rather than the plan, changed two
of them. This section is the result, and it is the reason steps 2, 3 and 5 are not in
the pull requests yet.

**Step 1 — done.** `VICTORY_JOIN_ONE_CALL=1`, off by default upstream, on in
`Play-VIctory-8GB.cmd` (#31). It is a wait removed, not a preparing.

**Step 2 — the lists' steps cannot be forked and joined, and this is the finding.**
`HostList::run_deferred` (`host.rs`, the loop over a list's `deferred().copies`) is not a
pure function of its list: inside it, in order, it calls `batch.fill()`,
`batch.flush_one(shadow)`, `crate::watch::wrote`, `crate::watch::ask_once_end`,
`crate::port_journal::prep_wrote` and `batch.title_written()`. That last one is the one
that matters: it means *"what was refreshed before this write is stale, refresh it
again"*. Those calls are a **serial log of writes**, not a set of independent jobs, and
splitting them across two threads changes which refreshes happen, not only when —
which is a rendering difference, not a timing one. Two threads in a row therefore need
**two batches and two journals with a stated merge point** (where `title_written` of one
half becomes a refresh for the other), before any thread is spawned. That is a design
first; the patch is the easy half, and it cannot be written from this machine where
the title cannot be run to see whether the log still holds.

**Step 3 — the lock is cheaper to describe than to take.** `self.submitted` is held from
preparation to copy-back because "snapshots and the ranges shaders write are per host
copy, not per submission". Letting the next submission be prepared while this one is
sent means two of those alive at once: `shadow::take_snapshots` and
`shadow::writing` would each have two sets. That is the memory cost the plan names, and
on a card with 6.44 GiB already shared it is the difference between a gain and a swap.
It is also the step where a wrong lock shows up as a rare lost write — the class of bug
this repository's own findings record as *six of two hundred runs*, i.e. one that will
not be caught by a single test run.

**Step 4 — largely already done, and the record says so.** The narrowing this step asks
for is what took 280 → 124 ms (descriptor records left to the draws), 69 → 6 ms (guards
looked at first) and 14 → 4-7 ms (AVX2 over the title's bytes). What is left of the
6 ms and 5 ms is the log above, not bytes looked at: there is no widened comparison
to narrow. I would not claim a gain here without a profile, and there is none on this
machine.

**Step 5 — a storage change, not a sleeve.** Viewing the card's own buffers where
fills copy them now means a `Shadow` (`Arc`, one per host copy) that sometimes has no
host copy at all, and every `Step::Refresh` and `batch.add` would have to answer what
to do when there is nothing on this side. It is the step with the largest measured
prize (~19 ms of Logan's 93) and the largest surface: `refresh_front`, `refresh_ahead`
and `apart()` all assume a host side exists.

**Step 6 — already upstream.** `VICTORY_OUTPUT` (the loader's patch) and
`VICTORY_TEST_LOWRES` for 540p; the launcher's ladder (#31) drives both.

### Step 2 as a design, before it is a patch

The loop cannot be forked wholesale. It can be **cut at a boundary**, and this is the
cut I would write:

1. **A boundary is a call that changes the log**: `batch.fill()`, `batch.flush_one`,
   `watch::wrote`, `watch::ask_once_end`, `port_journal::prep_wrote`,
   `batch.title_written()`, and the side effects of `Step::Run` (which may write
   anywhere and therefore ends the asking). Nothing else in the loop is one.
2. **The cut goes at the last boundary before the halfway step index**, and each half
   gets its own `Batch`, its own staging list and its own place in the journal. The
   second half does not start until the first half's batch has been flushed — the
   threads overlap the *work*, not the *log*.
3. **The merge point, stated as the invariant it has to keep**: a refresh collected
   before a title write must never be satisfied by bytes read after that write. So a
   half may only be cut where the two halves touch **no common `Shadow`** —
   `Arc::ptr_eq` over the two halves' copies, decided when the lists are handed over,
   not guessed per step. Where they do share one, the cut moves left.
4. **What proves it**: the same dump, the refresh count on the frame line, and a
   rendering comparison — not a timing. A wrong cut changes *which* refreshes happen,
   so it must be checked against the picture before it is checked against the clock.
5. **What it costs**: the `snapshot_bytes` column added to the CSV in this branch. If a
   submission's snapshots are a large part of 6.44 GiB shared, two of them is the
   answer to "can this be bought", and it is answered before the code is written.

What is measured today stands: **preparing is 93 ms against a 33 ms budget**, and a
frame cannot be shorter than the work of preparing it. Steps 2, 3 and 5 are the work
that would remove it. They are also the three that change what the title sees, and
none of them can be validated without the card they are for.

The honest current answer, with what is measured today: **30 fps is not reachable by
changing switches.** Preparing is 93 ms against a 33 ms budget, on a design whose own
record on a much faster card is 48 ms. It is reachable by removing about 60 ms of
work that exists, in the order above, and the first number that says whether the path
is real is `preparing_us` when the pieces per frame fall.
