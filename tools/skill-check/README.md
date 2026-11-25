# skill-check: frontmatter checks for Breach's skills (study exercise)

A study exercise for task statement 3.2 (commands and skills). The tests read every
`.claude/skills/*/SKILL.md` and check the frontmatter: a name that matches the folder, a
description, an `argument-hint` wherever the body reads `$ARGUMENTS` or `$N`, and, for
`find-decision`, a forked, read-only run. Run from the repo root with
`python3 -m unittest discover -s tools/skill-check -v` (needs PyYAML).
