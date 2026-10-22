"""Adds each open issue and PR that isn't yet in the owner's Project (spec 40 step 6,
Decision 156). Run by hand by .github/workflows/project-backfill.yml.

`gate` decides whether to act (the App is set up, as project-status) and writes `ready` to
$GITHUB_OUTPUT. `apply` lists the open issues and PRs with GITHUB_TOKEN through REST, lists
the Project's items once with the App's token, PROJECT_TOKEN, and adds what's missing.
Status is left to the Project's built-in workflow (Inbox). A second run adds nothing."""
import os
import sys

import project_status
from github_client import GitHubClient


class OpenItemsClient(GitHubClient):
    """The one REST read the backfill makes: every open issue and PR, with its node id."""

    def open_items(self):
        issues = [(i["number"], i["node_id"]) for i in self._list("issues?state=open") if "pull_request" not in i]
        pulls = [(p["number"], p["node_id"]) for p in self._list("pulls?state=open")]
        return issues + pulls


def gate(app_id, has_key):
    return project_status.gate("workflow_dispatch", "", app_id, has_key)


def apply(repo, writer, project_owner, project_number):
    project = writer.project(project_owner, project_number)
    held = writer.item_contents(project["id"])
    open_items = repo.open_items()
    missing = [(number, node) for number, node in open_items if node not in held]
    for number, node in missing:
        writer.add_item(project["id"], node)
        print(f"Added #{number}")
    print(f"{len(missing)} added, {len(open_items) - len(missing)} already there")


def main(mode):
    if mode == "gate":
        ready, message = gate(os.environ.get("PROJECT_APP_ID", ""), os.environ.get("HAS_APP_KEY") == "true")
        print(message)
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write(f"ready={'true' if ready else 'false'}\n")
        return
    from graphql_client import ProjectClient

    repo = OpenItemsClient.from_env()
    writer = ProjectClient(os.environ["PROJECT_TOKEN"])
    apply(repo, writer, os.environ["PROJECT_OWNER"], int(os.environ["PROJECT_NUMBER"]))


if __name__ == "__main__":
    main(sys.argv[1])
