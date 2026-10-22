"""An in-memory stand-in for github_client.GitHubClient, for the tests."""


class FakeClient:
    def __init__(self, labels=(), milestones=(), issues=()):
        self.labels = list(labels)
        self.milestones = {title: number for number, title in enumerate(milestones, 1)}
        self.milestone_states = {number: "open" for number in self.milestones.values()}
        self.records = {}
        # Issues the full listing misses but a milestone's listing finds (spec 40 step 4).
        self.hidden = set()
        self.parents = {}
        for number, title in issues:
            self.add_issue(title, number=number)
        self.pr_files = {}
        self.created_labels = []
        self.created_milestones = []
        self.reset_writes()

    def reset_writes(self):
        self.created_issues = []
        self.closed = []
        self.comments = []
        self.sub_issues = []
        self.reopened = []
        self.milestone_writes = []

    def add_issue(self, title, body="", labels=(), number=None, milestone=None, pull_request=False, merged=False, hidden=False):
        number = number if number is not None else 1000 + len(self.records)
        self.records[number] = {
            "number": number,
            "title": title,
            "body": body,
            "state": "open",
            "state_reason": None,
            "labels": list(labels),
            "milestone": milestone,
            "pull_request": pull_request,
            "merged": merged,
        }
        if hidden:
            self.hidden.add(number)
        return number

    def add_pull_request(self, number, labels=(), merged=False, body="", files=()):
        self.pr_files[number] = list(files)
        return self.add_issue(f"PR {number}", body, labels, number, pull_request=True, merged=merged)

    def number_of(self, title, prefix=False):
        for number, record in self.records.items():
            if record["title"] == title or (prefix and record["title"].startswith(title)):
                return number
        return None

    def list_labels(self):
        return list(self.labels)

    def create_label(self, name, color, description):
        self.created_labels.append(name)
        self.labels.append(name)

    def list_milestones(self):
        return dict(self.milestones)

    def create_milestone(self, title):
        self.created_milestones.append(title)
        self.milestones[title] = len(self.milestones) + 1
        self.milestone_states[self.milestones[title]] = "open"
        return self.milestones[title]

    def list_issues(self):
        return [(n, r["title"]) for n, r in self.records.items() if not r["pull_request"]]

    def list_issue_records(self):
        """Newest first, as GitHub lists them."""
        return [dict(self.records[n]) for n in sorted(self.records, reverse=True) if n not in self.hidden]

    def list_milestone_issue_records(self, milestone):
        return [dict(r) for n, r in sorted(self.records.items(), reverse=True) if r["milestone"] == milestone]

    def get_issue(self, number):
        return dict(self.records[number]) if number in self.records else None

    def create_issue(self, title, body, labels, milestone):
        number = self.add_issue(title, body, labels, milestone=milestone)
        self.created_issues.append({"title": title, "body": body, "labels": labels, "milestone": milestone})
        return number

    def set_milestone(self, number, milestone):
        self.records[number]["milestone"] = milestone

    def close_issue(self, number, reason):
        self.records[number].update(state="closed", state_reason=reason)
        self.closed.append((number, reason))

    def reopen_issue(self, number):
        self.records[number].update(state="open", state_reason="reopened")
        self.reopened.append(number)

    def list_milestone_records(self):
        return [{"number": n, "title": t, "state": self.milestone_states[n]} for t, n in self.milestones.items()]

    def get_milestone(self, number):
        return next(m for m in self.list_milestone_records() if m["number"] == number)

    def set_milestone_state(self, number, state):
        self.milestone_states[number] = state
        self.milestone_writes.append((number, state))

    def comment(self, number, body):
        self.comments.append((number, body))

    def get_parent(self, number):
        return self.parents.get(number)

    def add_sub_issue(self, parent, child, replace_parent=False):
        if child in self.parents and not replace_parent:
            raise ValueError(f"#{child} already has a parent")
        self.parents[child] = parent
        self.sub_issues.append((parent, child))

    def list_sub_issues(self, parent):
        return [dict(self.records[n]) for n, p in sorted(self.parents.items()) if p == parent]

    def list_pr_files(self, number):
        return list(self.pr_files.get(number, []))
