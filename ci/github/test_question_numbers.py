"""Tests for question_numbers.py: a question issue whose Q<n> is already held gets one
comment naming the holder and the next free number (spec 51 step 3, Decision 181)."""
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import question_numbers  # noqa: E402
from fake_client import FakeClient  # noqa: E402

ARCHIVE = """# Open questions

```markdown
### Q<n>. <the question, short>
```

## Open

## Closed

- Q30, track open questions on GitHub. **Resolved by Decision 148.**
- Q29, a side-by-side view. **Resolved.**
"""


def _client():
    client = FakeClient()
    client.add_issue("Q46. Numbers taken by two sessions", labels=["question"], number=884)
    client.add_issue("Q49. Something else", labels=["question"], number=890)
    client.add_issue("Spec 51: Number claims", number=895)
    return client


def _run(client, number, archive=ARCHIVE):
    with redirect_stdout(io.StringIO()):
        question_numbers.apply(client, number, archive)


class DuplicateTest(unittest.TestCase):
    def test_the_later_issue_is_told_who_holds_it_and_the_next_free(self):
        client = _client()
        client.add_issue("Q46. The same number again", labels=["question"], number=900)
        _run(client, 900)
        self.assertEqual(len(client.comments), 1)
        number, body = client.comments[0]
        self.assertEqual(number, 900)
        self.assertIn("Q46 is held by #884; the next free is Q50", body)

    def test_the_first_claimant_gets_no_comment(self):
        client = _client()
        client.add_issue("Q46. The same number again", labels=["question"], number=900)
        _run(client, 884)
        self.assertEqual(client.comments, [])

    def test_a_closed_question_still_holds_its_number(self):
        client = _client()
        client.close_issue(884, "completed")
        client.add_issue("Q46. The same number again", labels=["question"], number=900)
        _run(client, 900)
        self.assertIn("Q46 is held by #884", client.comments[0][1])

    def test_a_number_only_in_the_archive_names_the_archive(self):
        client = _client()
        client.add_issue("Q30. An old number", labels=["question"], number=901)
        _run(client, 901)
        self.assertEqual(len(client.comments), 1)
        self.assertIn("Q30 is held by `OPEN_QUESTIONS.md`'s Closed archive; the next free is Q50", client.comments[0][1])

    def test_a_second_label_event_adds_no_second_comment(self):
        client = _client()
        client.add_issue("Q46. The same number again", labels=["question"], number=900)
        _run(client, 900)
        _run(client, 900)
        self.assertEqual(len(client.comments), 1)

    def test_a_free_number_gets_no_comment(self):
        client = _client()
        client.add_issue("Q50. A new one", labels=["question"], number=902)
        _run(client, 902)
        self.assertEqual(client.comments, [])

    def test_an_issue_not_labelled_question_is_left_alone(self):
        client = _client()
        client.add_issue("Q46. Not a question issue", number=903)
        _run(client, 903)
        self.assertEqual(client.comments, [])

    def test_a_question_without_a_q_number_is_left_alone(self):
        client = _client()
        client.add_issue("Remind me when spec 52 starts", labels=["question"], number=904)
        _run(client, 904)
        self.assertEqual(client.comments, [])


class ArchiveTest(unittest.TestCase):
    def test_reads_entries_under_closed_only_outside_fences(self):
        self.assertEqual(question_numbers.archive_numbers(ARCHIVE), {29, 30})


if __name__ == "__main__":
    unittest.main()
