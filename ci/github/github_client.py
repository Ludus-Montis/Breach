"""The small REST client sync.py and migrate_questions.py use: list and create for labels,
milestones and issues, nothing more (spec 36, isp-fit)."""
import json
import os
import re
import urllib.request

API = "https://api.github.com"
NEXT_LINK = re.compile(r'<([^>]+)>;\s*rel="next"')


class GitHubClient:
    def __init__(self, repo, token):
        self.base = f"{API}/repos/{repo}"
        self.token = token

    @classmethod
    def from_env(cls):
        return cls(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_TOKEN"])

    def _request(self, method, url, payload=None):
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
        return self._request("POST", f"{self.base}/issues", payload)[0]["number"]
