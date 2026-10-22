"""The small REST client sync.py and migrate_questions.py use: list and create for labels,
milestones and issues (spec 36), and the issue writes the step issues and a spec PR's
milestone need (spec 40), nothing more (isp-fit)."""
import json
import os
import re
import time
import urllib.error
import urllib.request

API = "https://api.github.com"
NEXT_LINK = re.compile(r'<([^>]+)>;\s*rel="next"')
# GitHub's secondary rate limit: a second between writes. The first step-issue backfill
# makes a few hundred.
WRITE_PAUSE = 1.0


class GitHubClient:
    def __init__(self, repo, token):
        self.base = f"{API}/repos/{repo}"
        self.token = token
        self._ids = {}

    @classmethod
    def from_env(cls):
        return cls(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_TOKEN"])

    def _request(self, method, url, payload=None):
        if method != "GET":
            time.sleep(WRITE_PAUSE)
        data = json.dumps(payload).encode() if payload is not None else None
        request = urllib.request.Request(url, data=data, method=method)
        request.add_header("Authorization", f"Bearer {self.token}")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        with urllib.request.urlopen(request) as response:
            link = NEXT_LINK.search(response.headers.get("Link", ""))
            return json.load(response), link.group(1) if link else None

    def _list(self, path):
        url, items = f"{self.base}/{path}{'&' if '?' in path else '?'}per_page=100", []
        while url:
            page, url = self._request("GET", url)
            items.extend(page)
        return items

    def list_labels(self):
        return [label["name"] for label in self._list("labels")]

    def create_label(self, name, color, description):
        self._request("POST", f"{self.base}/labels", {"name": name, "color": color, "description": description})

    def list_milestones(self):
        return {m["title"]: m["number"] for m in self._list("milestones?state=all")}

    def create_milestone(self, title):
        return self._request("POST", f"{self.base}/milestones", {"title": title})[0]["number"]

    def list_issues(self):
        issues = self._list("issues?state=all")
        return [(i["number"], i["title"]) for i in issues if "pull_request" not in i]

    def create_issue(self, title, body, labels, milestone):
        payload = {"title": title, "body": body, "labels": labels}
        if milestone is not None:
            payload["milestone"] = milestone
        issue = self._request("POST", f"{self.base}/issues", payload)[0]
        self._ids[issue["number"]] = issue["id"]
        return issue["number"]

    @staticmethod
    def _record(issue):
        pull_request = issue.get("pull_request")
        return {
            "number": issue["number"],
            "title": issue["title"],
            "body": issue.get("body") or "",
            "state": issue["state"],
            "labels": [label["name"] for label in issue["labels"]],
            "milestone": issue["milestone"]["number"] if issue.get("milestone") else None,
            "pull_request": pull_request is not None,
            "merged": bool(pull_request and pull_request.get("merged_at")),
        }

    def list_issue_records(self):
        """Every issue and PR, once: what step_issues.py reads its own issues from."""
        return [self._record(issue) for issue in self._list("issues?state=all")]

    def list_milestone_issue_records(self, milestone):
        """A milestone's issues and PRs: the sync's second read before it creates (spec 40 step 4)."""
        return [self._record(issue) for issue in self._list(f"issues?milestone={milestone}&state=all")]

    def get_issue(self, number):
        try:
            return self._record(self._request("GET", f"{self.base}/issues/{number}")[0])
        except urllib.error.HTTPError as error:
            if error.code in (404, 410):
                return None
            raise

    def set_milestone(self, number, milestone):
        self._request("PATCH", f"{self.base}/issues/{number}", {"milestone": milestone})

    def close_issue(self, number, reason):
        self._request("PATCH", f"{self.base}/issues/{number}", {"state": "closed", "state_reason": reason})

    def comment(self, number, body):
        self._request("POST", f"{self.base}/issues/{number}/comments", {"body": body})

    def get_parent(self, number):
        """The issue's parent's number, or None if it has none."""
        try:
            return self._request("GET", f"{self.base}/issues/{number}/parent")[0]["number"]
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            raise

    def add_sub_issue(self, parent, child, replace_parent=False):
        if child not in self._ids:
            self._ids[child] = self._request("GET", f"{self.base}/issues/{child}")[0]["id"]
        payload = {"sub_issue_id": self._ids[child]}
        if replace_parent:
            payload["replace_parent"] = True
        self._request("POST", f"{self.base}/issues/{parent}/sub_issues", payload)

    def list_pr_files(self, number):
        return [f["filename"] for f in self._list(f"pulls/{number}/files")]
