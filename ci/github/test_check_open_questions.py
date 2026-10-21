"""Tests for check_open_questions.py (spec 36 step 2): no entry under Open."""
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_open_questions  # noqa: E402

ENTRY_UNDER_OPEN = """# Open questions

```markdown
### Q<n>. <the question, short>
```

## Open

Open questions are issues labelled `question`.

### Q31. A new question
**Raised:** today · **Raise when:** Next design session

## Closed

### Q20. An old one
"""


def check(text):
    out = io.StringIO()
    with redirect_stdout(out):
        code = check_open_questions.check(text, "OPEN_QUESTIONS.md")
    return code, out.getvalue()


class CheckOpenQuestionsTest(unittest.TestCase):
    def test_an_entry_under_open_fails_the_check(self):
        code, out = check(ENTRY_UNDER_OPEN)
        self.assertEqual(code, 1)
        self.assertIn("OPEN_QUESTIONS.md:11:", out)
        self.assertIn("### Q31.", out)

    def test_the_pointer_file_passes(self):
        text = check_open_questions.QUESTIONS.read_text(encoding="utf-8")
        self.assertEqual(check(text)[0], 0)

    def test_entries_under_closed_and_in_a_fence_pass(self):
        text = ENTRY_UNDER_OPEN.replace("### Q31. A new question", "Moved to #300.")
        self.assertEqual(check(text)[0], 0)


if __name__ == "__main__":
    unittest.main()
