# Spec 35: The unit store
## Purpose
## Status
## The store
## Identity
## Steps
## Scenarios
## Test-first order (per step)
## Rubric answers
## Settled for step 1 (the user, 2026-10-14)
## Settled for step 5 (the user, 2026-10-15; Decision 145)
## Settled for step 6 (the user, 2026-10-16; Decision 146)
## Settled for step 7 (the user, 2026-10-17; Decision 147)
## Built
### Step 0, the skeleton
### Step 1a, `position`
### Step 1b, `at`, `offset` and the flag bits
### Step 1c, the Rust backend owns them
### Step 2a, the entries' other keys
### Step 2b, the Rust backend owns them
### Step 3, bearing
### Step 4a, rank and column
### Step 4b, the seek on the store
### Step 4c, the walk on the store
### Step 4d, the facing on the store
### Step 4e, the sync reads the store
### Places read once a pass (after step 4)
### Store reads held once a pass (Decision 144)
### Step 5a, the lane skirmish retired
### Step 5b, hp and cooldown in the store
### The rout's columns held for its pass (after step 5b)
### Step 6a, `UnitView`
### Step 6b, the slower phases on the view
### Step 6c, stamina, speed and exertion
### Step 7a, one writer for state and the kept lists
### Step 7b, the downed's fields and recovery's
### Step 7c, the rest
## Open, for the implementing session to settle and record here
