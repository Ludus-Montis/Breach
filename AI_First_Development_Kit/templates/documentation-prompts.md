---
doc: templates/documentation-prompts
status: active
---

# Agent Prompts for Documentation Phases

Paste or reference these in the agent's task instructions. Each assumes the implementation is
complete. Paths are as in `principles/documentation-phases.md`: kit-relative, except
`README.md`, `CHANGELOG.md` and `docs/` at the project root.

## Impact assessment
> Read the diff and the table in `principles/documentation-phases.md`. Fill in
> `templates/doc-impact-assessment.md`. Do not edit any docs yet. Output only the completed
> assessment.

## Phase 1: README
> Using the impact assessment, update `README.md` per Phase 1 of
> `principles/documentation-phases.md`. Keep it Standard Readme compliant. Do not add detail
> that belongs in `docs/`; link to it instead. Verify every command you write or keep. Run
> `scripts/check-docs.sh readme` and fix any failures.

## Phase 2: Changelog
> Using the impact assessment and the commit messages since the last tag, add entries under
> `## [Unreleased]` in `CHANGELOG.md` per Phase 2. Write for users. Mark breaking changes.
> Run `scripts/check-docs.sh changelog` and fix any failures.

## Phase 3: Documentation
> Using the impact assessment, create or update pages under `docs/` per Phase 3, placing each
> in the correct Diátaxis quadrant. Update `docs/README.md`. Propose a Decision (or, in a
> project without a decision ledger, an ADR) for any architectural decision; a human
> authorises it. Run `scripts/check-docs.sh docs` and fix any failures.

## Completion
> Add the Documentation block to the PR description, then run `scripts/check-docs.sh all`.
> Do not report the task complete until it passes.
