"""Fails when an entry (`### Q...`) appears under OPEN_QUESTIONS.md's Open section.

Spec 36 step 2, Decision 148: open questions are issues labelled `question`; the file keeps
only a pointer and its Closed archive. Exits 1 naming each offending line.
"""
import sys
from pathlib import Path

QUESTIONS = Path(__file__).resolve().parents[2] / "OPEN_QUESTIONS.md"


def check(text, name):
    failures, section, fenced = 0, None, False
    for number, line in enumerate(text.splitlines(), 1):
        if line.startswith("```"):
            fenced = not fenced
        elif fenced:
            continue
        elif line.startswith("## "):
            section = line[3:].strip()
        elif section == "Open" and line.startswith("### Q"):
            print(f"{name}:{number}: {line.strip()}")
            failures += 1
    if failures:
        print("check_open_questions: open questions are issues labelled `question` (Decision 148).")
        return 1
    return 0


def main():
    return check(QUESTIONS.read_text(encoding="utf-8"), QUESTIONS.name)


if __name__ == "__main__":
    sys.exit(main())
