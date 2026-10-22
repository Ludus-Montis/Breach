"""Sets a PR's Status in the owner's Project, and the same Status on each issue it closes
(spec 40 step 3, Decision 155). Run by .github/workflows/project-status.yml.

`gate` decides whether to act at all (the App is set up; a label event is for `handoff`)
and writes `ready` to $GITHUB_OUTPUT. `apply` reads the PR with GITHUB_TOKEN and writes the
Project with the App's token, PROJECT_TOKEN. Merged or closed is the Project's own
built-in workflows' (Done)."""
import json
import os
import sys

HANDOFF = "handoff"
LABEL_EVENTS = ("labeled", "unlabeled")


def status_for(draft, labels):
    """The rule: a parked hand-off is Next, other drafts In progress, a ready PR In review."""
    if not draft:
        return "In review"
    return "Next" if HANDOFF in labels else "In progress"


def gate(action, label, app_id, has_key):
    if action in LABEL_EVENTS and label != HANDOFF:
        return False, f"The `{label}` label doesn't change the Status; nothing to do."
    if not app_id or not has_key:
        return False, "The Project's GitHub App isn't set up (PROJECT_APP_ID, PROJECT_APP_PRIVATE_KEY); skipping."
    return True, "Setting the Project's Status."


def _set(writer, project, content_id, option):
    item = writer.item_id(project["id"], content_id) or writer.add_item(project["id"], content_id)
    writer.set_status(project["id"], item, project["field"], option)


def apply(reader, writer, repo, number, project_owner, project_number):
    owner, name = repo.split("/")
    pr = reader.pull_request(owner, name, number)
    status = status_for(pr["draft"], pr["labels"])
    project = writer.project(project_owner, project_number)
    option = project["options"].get(status)
    if option is None:
        names = ", ".join(project["options"])
        raise SystemExit(f"The Project's Status field has no option `{status}` (it has: {names}); add it (spec 40).")
    for content_id in [pr["id"], *pr["closing"]]:
        _set(writer, project, content_id, option)
    print(f"PR #{number} and {len(pr['closing'])} closing issue(s): {status}")


def _event():
    with open(os.environ["GITHUB_EVENT_PATH"], encoding="utf-8") as handle:
        return json.load(handle)


def main(mode):
    event = _event()
    if mode == "gate":
        label = (event.get("label") or {}).get("name", "")
        ready, message = gate(event["action"], label, os.environ.get("PROJECT_APP_ID", ""), os.environ.get("HAS_APP_KEY") == "true")
        print(message)
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write(f"ready={'true' if ready else 'false'}\n")
        return
    from graphql_client import ProjectClient

    reader = ProjectClient(os.environ["GITHUB_TOKEN"])
    writer = ProjectClient(os.environ["PROJECT_TOKEN"])
    number = event["pull_request"]["number"]
    apply(reader, writer, os.environ["GITHUB_REPOSITORY"], number, os.environ["PROJECT_OWNER"], int(os.environ["PROJECT_NUMBER"]))


if __name__ == "__main__":
    main(sys.argv[1])
