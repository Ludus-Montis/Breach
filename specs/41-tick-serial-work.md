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
