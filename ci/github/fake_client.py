"""An in-memory stand-in for github_client.GitHubClient, for the tests."""


class FakeClient:
    def __init__(self, labels=(), milestones=(), issues=()):
        self.labels = list(labels)
        self.milestones = {title: number for number, title in enumerate(milestones, 1)}
        self.issues = [(number, title) for number, title in issues]
        self.created_labels = []
        self.created_milestones = []
        self.created_issues = []

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
        return self.milestones[title]

    def list_issues(self):
        return list(self.issues)

    def create_issue(self, title, body, labels, milestone):
        number = 1000 + len(self.issues)
        self.issues.append((number, title))
        self.created_issues.append(
            {"title": title, "body": body, "labels": labels, "milestone": milestone}
        )
        return number
