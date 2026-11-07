---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 41: The tick's serial work, outcomes unchanged

## Purpose

Spec 39 step 10 brought the 25k tick (24,576 a side, 24 squads of 1,024) from about 566 ms
to about **205 ms** (95th percentile 241; `bench` run #77, 10d-i, an EPYC 9V45 with 4
threads). Decision 159's target is a 95th-and-99th-percentile tick of **50 ms**, with an
interim bar of 100 ms. Step 10's last options that change outcomes were weighed in #738:
- the cadence and `f32` columns were parked (#739, items 1 and 7);
- a different reduction order turned out not to be needed (#739, item 9).

This spec takes what is left that changes **no outcome**:
- work that runs serially but needn't;
- work that is repeated;
- work done through slow general-purpose machinery (std's SipHash maps, a global sort,
  per-unit allocations).

It found these by reading the hot passes' code. Its first step measures inside the phases,
so the rest are judged on numbers.

Where the time went in run #77 (ms, 25k mean):

| Phase | ms | Share |
|---|---|---|
| `scrum: seek, walk and facing` | 92 | 45% |
| `bodies (parting)` | 42 | 20% |
| `rout` | 12 (95th 36) | 6% |
| `wounds (tend)` | 9 | 4% |
| `units walk to places` | 9 | 4% |
| `scrum: engage, prepare, stance` | 8 | 4% |
| `roster` | 6 | 3% |
| `fight (blows)` | 6 | 3% |
| the other 21 phases | 21 | 10% |

**Settled by Decision 159** (its order: work that follows change, then threads; its threads
pattern) and **Decision 151** (what guards outcomes). Every step keeps outcomes identical, so
none needs a Decision of its own, `Outcomes-changed:` or Gate 2. A step that finds it can't
keep them identical stops and asks the owner.

## Status

Agreed in the design session of 2026-11-06 (the owner: "Sounds like a plan, lets get that
written up as a new spec to follow"), after spec 39 step 10f was settled in #738. Four steps,
each its own PR, built with `/run-phase` as "spec 41 step N".
- Step 1 first: its measurements may reorder 2 to 4, and the step that builds next says so.
- Step 2 is replaced by spec 42 step 2 (Decision 161), which starts once step 1 is in.
- Step 3 changes `src/parting.rs`, as spec 39 step 10f does. It branches from 10f's branch
  while 10f is unmerged, or from `main` once 10f has merged.
- This spec's PR is stacked on spec 39 step 10's design (#724), which holds Decisions 159 and
  160. Merge it after #724.

## Rules for every step

- **Outcomes identical.** CI's `sim-checks` digests and trials equal the base's, with
  threads on and off (spec 39 step 10c-i's both-ways check).
- **Benched** against the step's base with `rounds` of 5 or more at both sizes (`preset=4096`
  and `preset=25k`), numbers in the PR. Compare ticks only within one run: `bench`'s runners
  differ by about 50% (spec 39, "Step 10e").
- **Parallel work** follows Decision 159's pattern:
  - map on the pool into buffers, then apply in squad and member order;
  - any reduction in a fixed order;
  - the result doesn't depend on the number of threads;
  - a pass asks `threaded.on(units)` (`src/threads.rs`).
- **Nothing by id or list order** (Decision 97). Where a step replaces a structure, ties
  break as they did.

## Steps

### Step 1, measuring inside the phases

The phases are timed as wholes, so the 92 ms of `seek, walk and facing` can't be split three
ways, and rout's 36 ms spikes can't be traced to a part.
- **Laps inside the big phases:**
  - `scrum: seek, walk and facing` becomes three laps, `scrum: seek`, `scrum: walk` and
    `scrum: facing`. The tick already returns `seek_usec`; it becomes the seek lap.
  - `rout` gets laps for its parts (the flight, the rallies' searches, the regroup's enemy
    search, the counting and re-forms), as `src/rout/` divides them.
  - `bodies (parting)` gets laps for the pair search, the pushes, and the sums and
    applying.
  - The lap names are listed in the spec when built, and `bench_compare.py` reads them like
    any other.
- **A sampling profile in `bench`:**
  - a `profile` input (default off) runs the head once more under
    `perf record --call-graph dwarf` (Linux `perf`, installed on the runner, with
    `kernel.perf_event_paranoid` lowered through `sudo sysctl`);
  - the crate gets a `profiling` build profile (`release` with `debug = 1`);
  - the summary prints `perf report --no-children --percent-limit 0.5` between its own
    markers: the functions that take the time, flat;
  - if `perf` can't run on the runner, the step names why and uses `samply` or a
    timer-based alternative, recorded here.
- **Record the first profile** of the 25k preset in this spec, under "Measured". Steps 2 to
  4 are checked against it, and the measured order decides which of them comes next.

#### Measured (step 1, 2026-11-06)

Built as spec'd, with one choice the spec left open: a part ends with `laps::split`, which
times it but shows a watcher nothing, and only the phase's last part ends with `lap`. So
`counts=1` and the cuts still see the state once a phase, under its last part's name (`scrum:
facing`, `rout: listing`, `bodies: commit`). Shown at every part, the cuts' coverage check
failed: `rout: breaks` and `rout: re-forms` change the state in none of the committed cuts. The
cuts' prints are unchanged; only those three laps are renamed.

The laps, in the order the tick runs them:
- `scrum: seek` (the sync and `seeking::seek`; `seek_usec` and `breach_native_bench`'s `seek`),
  `scrum: walk` (the walk and the ground's pay), `scrum: facing`.
- `rout: flight`, `rout: rallies`, `rout: regroup's search`, `rout: re-forms`, `rout: breaks`
  (only on ticks something routs), then `rout: listing` (the roster listed again).
- `bodies: sync`, then each pass's `bodies: pair search`, `bodies: pushes` and `bodies: sums
  and applying`, then `bodies: commit` (the loosened and the moved written back).

`bench` runs [#87](https://github.com/Ludus-Montis/Breach/actions/runs/42157496722) (`preset=25k`, `profile` on, Xeon 8370C) and
[#88](https://github.com/Ludus-Montis/Breach/actions/runs/42157500121) (`preset=4096`, EPYC 7763), 5 rounds each against `main`:
- **No cost:** the tick 316.9 against 319.0 ms at 25k (-0.7%) and 50.7 against 50.8 at 4096
  (-0.2%), ranges overlapping; no phase present on both sides moved outside its range.
- **The parts sum to the old laps** (round 1, 25k, ms): scrum 123.9 + 19.2 + 5.3 = 148.4 against
  148.6; parting 4.0 + 46.1 + 5.1 + 5.7 + 1.5 = 62.4 against 62.9; rout 9.9 + 7.8 + 1.3 + 0.0 +
  1.1 + 0.2 = 20.3 against 20.3.

The 25k series' parts (round 1 of #87, ms):

| Part | Mean | 95th |
|---|---|---|
| `scrum: seek` | 123.9 | 154.0 |
| `bodies: pair search` | 46.1 | 48.0 |
| `scrum: walk` | 19.2 | 22.5 |
| `rout: flight` | 9.9 | 44.3 |
| `rout: rallies` | 7.8 | 16.6 |
| `bodies: sums and applying` | 5.7 | 6.3 |
| `scrum: facing` | 5.3 | 6.3 |
| `bodies: pushes` | 5.1 | 5.4 |
| `bodies: sync` | 4.0 | 5.9 |
| `bodies: commit` | 1.5 | 1.6 |
| `rout: regroup's search` | 1.3 | 3.0 |
| `rout: breaks` | 1.1 | 4.2 |
| `rout: listing`, `rout: re-forms` | 0.2, 0.0 | 0.3, 0.1 |

So seek's 92 ms (run #77) is almost all the seek itself, and rout's spikes are its flight
(44 ms at tick 104) and rallies (17 ms).

**The first profile** (#87, `perf` worked on the runner as built: `linux-tools-$(uname -r)`,
`task-clock` samples at 99 Hz, about 3,000 of them). It samples the tool's whole run, the
march and settling before contact as well as the timed series, so its shares are of the run,
not of the series' tick. Functions over 1%, self time:

| Share | Function |
|---|---|
| 30.2% | `parting::pairs::{closure#1}` (each body's neighbour search) |
| 8.6% | `scrum::Search::squads::{closure#0}` (building the rings) |
| 3.4% | `walking::walk::{closure#1}` |
| 2.9% | `Field::sync` |
| 2.5% | `scrum::Plan::consider` |
| 2.4% | `march::sync::step` |
| 2.3% | `tick::step` |
| 2.0% | `scrum::Plan::pick` |
| 2.0% | `grid::Grid::build` |
| 1.9% | `rout::flight::step` |
| 1.8% | `parting::pass::step` |
| 1.7%, 1.2% | `floor`, `floorf` (libm) |
| 1.6% | `BTreeMap<usize, Member>::bulk_build_from_sorted_iter` |
| 1.6% | `Vec<Listed>::clone` (the roster cloned) |
| 1.6%, 1.2% | `det_math::sin`, `det_math::cos` |
| 1.4% | `groups::pass::meet` |
| 1.2% | `Vec<Recovering>::from_iter` (`recovery::pass::ward`) |
| 1.2% | `parting::push` |
| 1.1% | `moving::share::pourers` |

Below 1%: `realloc`, page faults, `Frame::unit_rect`, `play::listed`, `scrum::seek::step`,
rayon's parallel sort of the pairs (0.6%), `det_math::atan`, `fight::choose`,
`mustering::listing`.

**What this says about the order.** Seek is first by far, and it is spec 42 step 2's (which
replaces step 2 here), so that comes next. Parting's pair search (46 ms, and the profile's
largest function) is step 3. The roster clones, `BTreeMap` builds, `realloc` and libm's `floor`
are step 4's ground. Rout's flight and rallies spikes are spec 42 step 3's. A `task-clock`
profile can't tell waiting on memory from arithmetic (the runner's VM gives no hardware
counters), so it doesn't reopen anything under "Not in this spec".

### Step 2, seek's search off the serial loop

> **Replaced by spec 42 step 2** (Decision 161, 2026-11-06): seek's search and its claims
> come off the serial loop together there, settled in rounds. Not built here.

`scrum::plan` takes seekers one at a time in contest order (`kept`, then `pick`, `aim` and
`press`), each reading the claims made before it. Only building the rings (`Search::squads`)
is on the pool. `pick` walks each seeker's rings of candidate slots, and for each candidate
`consider`:
- works out its key: the distance, how far ahead, the distance to its place, then the foe's
  draw and the mirror roll;
- checks the leash and the ground (`within`);
- queries the crowd's grid for a body on the point;
- checks `Claims` (a `HashMap<(i32, i32), Vec<V2>>`).

All of it is serial. **Only the claims check depends on what earlier seekers did.**

- **The pattern (inspector–executor):**
  - **On the pool**, each seeker ranks its candidates. It applies every test that doesn't
    depend on claims: the ring's status at the start of the pass, `within`, no body on the
    point. It sorts them by the same total order `consider` uses (key, then draw, then the
    roll on a tie, then slot).
  - **Serially, in contest order**, each seeker takes the first slot in its list that is
    neither claimed (`Claims::near`) nor marked dead, then aims or presses as now.

  The serial pass then picks exactly the slot the current code picks, because `consider`'s
  early stops only prune slots that couldn't win.
- **Bounded lists:** the parallel ranking can't stop at the best found so far, as the serial
  search does. So each seeker's list is capped at K candidates. A seeker whose K are all
  claimed falls back to the serial search from where its list ended. K is a constant chosen
  from step 1's numbers and recorded here; it changes speed, never outcomes.
- **`within` stays pure.** It is the ground's callback (`&mut dyn FnMut`). The step makes it
  callable from the pool (`Fn + Sync`), or precomputes what it reads.
- **The crowded fallback** (`pick(seeker, true)`, then `press`) stays serial. Step 1's laps
  say how often it runs.
- **Smaller things in the same pass:**
  - `Claims` becomes a flat grid of claimed points, built as `src/grid.rs` builds its cells,
    rather than a SipHash map of `Vec`s;
  - `pick` no longer clones each seeker's owned slots (`owned.get(&body).cloned()`).
- **Kept serial** (#739, item 4): the claims themselves, and which seeker claims first. That
  is the contest. Settling it in parallel is a design subject of its own.

### Step 3, parting's pair search

Parting (`src/parting.rs`) runs `bodies_passes` passes a tick (3, `battle_tuning.json`).
Each pass searches for overlapping pairs afresh in `pairs`:
- **Buckets:** the bodies go into `Buckets` (a `HashMap<(i32, i32), Vec<u32>>` with std's
  SipHash), built serially, and each body looks up 9 buckets.
- **Allocations:** each body's search allocates its own `Vec` of pairs, then they are
  collected.
- **A global sort:** the pairs are sorted globally (`par_sort_unstable`).

The step changes it in four ways:
- **The flat grid:** buckets from `src/grid.rs`'s counting sort (or a grid of the same shape
  at `BUCKET` size), with no hashing.
- **No global sort:** each body's pairs come out in body order already (`into_par_iter`
  keeps the order). Sorting each body's short list by partner gives the same order as the
  global sort.
- **No per-body `Vec`:** count, then fill one buffer (two passes), or keep a reused buffer
  per thread.
- **A neighbour list across passes (Verlet skin):** search once a tick with a margin (the
  skin) added to the overlap test. Later passes test only the listed pairs, keeping the same
  order. The list is rebuilt whenever any body has moved more than half the skin since it
  was built, so every overlapping pair is always listed and the pushes are the same. The
  skin is a constant chosen from step 1's numbers and recorded here.
- Spec 39 step 10f (parting's per-body sums on the pool) lands first. Step 3 keeps its
  orders exactly: the pair order, and the order bodies first appear.

#### Built (step 3, 2026-11-07)

Built as spec'd in `src/parting/pairs.rs`, on `main` (spec 39 step 10f closed with parting's
sums measured and not kept, so there were no orders of its to keep). The choices the spec left
open:
- **The flat grid** is `src/grid.rs`'s own `Grid`: its `CELL` is 2, as `UnitBodies.BUCKET` is,
  and its cells hold their indices ascending, as the buckets did. `BUCKET`, parting's own
  hasher (`Fx`) and `Buckets` are gone.
- **No per-body `Vec`:** each chunk of 1,024 bodies (`CHUNK`) fills one buffer, each body's
  partners sorted in place, and the chunks are joined in order. That is the "reused buffer"
  option, with one allocation per chunk and no dependence on the number of threads.
- **The skin** (`SKIN`) is 0.25 cells. Step 1 gave the pair search as a whole (46 ms over 3
  passes) but no movement per pass, so 0.25 is a judgement: about half a grem's radius. A
  pass's pushes move a body by a share of its overlaps, so the list should last a tick's three
  passes. The listed pairs grow by about half again for two grems ((1.25 / 1)²). It changes speed only; `bench` judges it.
- **The list holds more than the overlap test needs.** It lists every pair within their radii
  plus the skin of each other, searched as far as the body's radius, the widest radius, the
  skin and `grid::MARGIN`, without the frames' test. A pass applies UnitBodies._pairs' whole
  test to each listed pair, in list order: buckets next to each other (it matters only for
  bodies wider than a bucket together), not in two squads' frames (flags change between
  passes), overlapping.
- **Rebuilt** before a pass when any body's position has moved more than half the skin, less
  `grid::MARGIN`, since the list was built. The margin is far above float rounding, so a pair
  that overlaps is always listed. The list is a tick's: `part` starts a new one.

Tests: `parting/pairs/tests.rs` (a crowd of 3,000 with bodies wider than a bucket, against a
search of every pair, on one thread and on the pool; the list kept while bodies move less
than half the skin, with some made loose, and rebuilt after one moves further). The existing
`parting` tests and the digests guard the outcomes.

### Step 4, the crate's general habits

- **A faster hasher that stays randomly seeded:**
  - every `HashMap`/`HashSet` in the crate moves to a fast hasher with a seed per process
    (`foldhash`'s or `ahash`'s `RandomState`) rather than std's SipHash;
  - a random seed keeps today's guard: any code that came to depend on a map's iteration
    order would show as digests differing between runs;
  - a fixed-seed hasher (FxHash) would hide that, so it's ruled out;
  - `clippy.toml`'s `disallowed-types` keeps std's `HashMap` and `HashSet` out of the
    crate's own code from then on.
- **mimalloc as the global allocator** (the `mimalloc` crate) in the library and the
  binaries. Rayon threads allocate and free in parallel, and the system `malloc`'s locks are
  shared. The step checks `native/build.sh` builds it on Windows, macOS and Linux, and says
  so in `native/README.md`.
- **Throwaway work each tick:**
  - `tick::step` clones the whole roster (`roster.clone()`) and relists it 4 times.
    `relisted` takes it by value and updates what changed, without the clone.
  - The buffers each pass builds anew and drops every tick (the seekers, the crowd's grid,
    parting's buffers) are kept on the field between ticks and cleared. A buffer is scratch
    space, not state: the snapshot doesn't write it, and a field read back from a snapshot
    starts it empty, to the same results.
- **As built** (the owner, 2026-11-07: leave parting alone if step 3 hasn't merged by the
  time step 4 is done; it hadn't):
  - the hasher is `foldhash`'s `fast::RandomState`, behind `src/hashing.rs`'s aliases;
  - the kept buffers are the field's `buffers` (`src/buffers.rs`): the seek's crowd (its
    bodies, points and grid), its claims and its seekers. `Grid::rebuild` refills a grid in
    its own buffers;
  - parting's `Buckets` (std's map with its own fixed hasher) and its buffers are left to
    step 3, behind a `clippy::disallowed_types` allow. Step 3 removes the allow with the map,
    and keeps its buffers in the field's `buffers`.

## Not in this spec

- **`target-cpu` x86-64-v3 (AVX2):** it could vectorise parting's arithmetic, and would stay
  deterministic (Rust never fuses `a * b + c`). But it raises the game's minimum CPU, which
  is the owner's call. Raise it only if step 1's profile shows arithmetic-bound loops.
- **Packing hot fields together** (position, radius, flags and squad in one array for the
  neighbour loops), and **merging passes** (walk and facing in one loop): only if step 1's
  profile shows the tick waiting on memory. It is #739 items 6 and 7's condition.
- **Settling contests in parallel** (#739, items 4 and 5): seek's claims, and the rout's and
  tend's landings. They change what units decide. If the tick is still over the interim bar
  after this spec, that's the next design session.
- **Anything that changes outcomes:** see #739 and #740.

## Scenarios

### Step 1
- Given the 25k preset, when `breach_tick_phases` runs, then `scrum: seek`, `scrum: walk`
  and `scrum: facing` are laps of their own, and their sum is within timing noise of the old
  single lap. The same holds for rout's and parting's parts.
- Given a `bench` run with `profile` on, when it finishes, then its summary lists the
  functions over 0.5% of the samples, between profile markers.
- Given the step's commit, when the digests run, then they equal the base's: laps change no
  outcome.

### Step 2
- Given a scrum (the digests' battles, and a seek fixture with seekers contesting the same
  slots), when it is planned with the ranking on the pool, then every seeker's code, goal
  foe, goal slot, next point and foe point equal the serial plan's.
- Given seekers whose first K candidates are all claimed by earlier seekers, when they
  pick, then they fall back and choose what the serial search chooses.
- Given the same scrum with threads on and off, and with K set to 1 and to unbounded, then
  the plans are identical.
- Given two candidates tied on key and draw, then the roll and then the slot break the tie,
  as in `consider`.

### Step 3
- Given bodies in a crowd, when the pairs are found with the flat grid and per-body sorts,
  then they are the same pairs, in the same order, as the global sort's.
- Given a neighbour list built at the tick's first pass, when bodies move less than half the
  skin, then later passes push the same pairs as a fresh search would. When a body moves
  further, the list is rebuilt before the pass.
- Given the digests and the both-ways check, then outcomes are identical.

### Step 4
- Given a map keyed as before, when it is iterated in two processes, then the order differs
  (the seed is random) and the digests don't.
- Given the crate's own code, when clippy runs, then std's `HashMap` and `HashSet` are
  refused outside the hasher's alias module.
- Given a tick stepped from a field read back from a snapshot, and the same tick on the
  original field (whose buffers are warm), then they are identical.

## Test-first order

1. Step 1: a cargo test red on `tick_phases`' lap names for the split laps, then the laps;
   then the bench workflow's `profile` input (checked by a run, not a test).
2. Step 2: a cargo test red on the ranked plan equalling the serial plan (fixture with
   contested slots, K = 1 and unbounded), then the ranking; then the fallback scenario; then
   `Claims` as a flat grid.
3. Step 3: red on the flat grid's pairs equalling the global sort's, then the grid; then the
   neighbour list's rebuild scenario, then the list.
4. Step 4: red on the snapshot-read scenario with kept buffers, then the buffers; the hasher
   and allocator are guarded by the digests and clippy.

## Rubric answers

- **single-noun-phrase:** the lap split and the profile; the ranked seek search; the pair
  search's neighbour list; the crate's hasher, allocator and kept buffers.
- **ocp-extension-point:** a new lap is a name at the call site; a new hot map takes the
  hasher alias; a new pass's scratch buffer joins the field's buffers.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the ranking takes only what `consider` reads that doesn't depend on claims;
  the neighbour list only positions, radii, flags and squads.
- **dip-direction:** passes depend on the grid and the hasher alias, never on a concrete map
  type.
- **general-over-special:** general. One pattern (rank in parallel, settle serially) for
  every seeker, and one neighbour-list rule for every body. No special cases.
- **order-independent:** the contest order is kept as is; the ranking's total order is
  `consider`'s; parallel results apply in squad and member order and don't depend on the
  number of threads (Decisions 97 and 159).
- **bounded-work:** this spec bounds the serial part of the tick. The ranking is K per
  seeker; the neighbour list is rebuilt only when movement demands it; no per-tick clone of
  the roster.
