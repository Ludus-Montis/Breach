"""The small GraphQL client project_status.py uses: read a PR, find a Project's Status
field, and find, add and set an item (spec 40 step 3), nothing more (isp-fit). One instance
per token: GITHUB_TOKEN reads the PR, the App's token writes the Project."""
import json
import urllib.request

API = "https://api.github.com/graphql"

PULL_REQUEST = """
query($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      id
      isDraft
      labels(first: 100) { nodes { name } }
      closingIssuesReferences(first: 50) { nodes { id } }
    }
  }
}"""

PROJECT = """
query($owner: String!, $number: Int!) {
  repositoryOwner(login: $owner) {
    ... on ProjectV2Owner {
      projectV2(number: $number) {
        id
        field(name: "Status") { ... on ProjectV2SingleSelectField { id options { id name } } }
      }
    }
  }
}"""

ITEMS = """
query($id: ID!) {
  node(id: $id) {
    ... on Issue { projectItems(first: 50) { nodes { id project { id } } } }
    ... on PullRequest { projectItems(first: 50) { nodes { id project { id } } } }
  }
}"""

ADD_ITEM = """
mutation($project: ID!, $content: ID!) {
  addProjectV2ItemById(input: {projectId: $project, contentId: $content}) { item { id } }
}"""

SET_STATUS = """
mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(
    input: {projectId: $project, itemId: $item, fieldId: $field, value: {singleSelectOptionId: $option}}
  ) { projectV2Item { id } }
}"""


class ProjectClient:
    def __init__(self, token):
        self.token = token

    def _query(self, query, **variables):
        data = json.dumps({"query": query, "variables": variables}).encode()
        request = urllib.request.Request(API, data=data, method="POST")
        request.add_header("Authorization", f"Bearer {self.token}")
        with urllib.request.urlopen(request) as response:
            body = json.load(response)
        if body.get("errors"):
            raise SystemExit(f"GraphQL: {body['errors']}")
        return body["data"]

    def pull_request(self, owner, repo, number):
        pr = self._query(PULL_REQUEST, owner=owner, repo=repo, number=number)["repository"]["pullRequest"]
        return {
            "id": pr["id"],
            "draft": pr["isDraft"],
            "labels": [label["name"] for label in pr["labels"]["nodes"]],
            "closing": [issue["id"] for issue in pr["closingIssuesReferences"]["nodes"]],
        }

    def project(self, owner, number):
        project = (self._query(PROJECT, owner=owner, number=number)["repositoryOwner"] or {}).get("projectV2")
        if not project or not project.get("field"):
            raise SystemExit(f"No Project {owner}/{number} with a Status field the App can read.")
        field = project["field"]
        return {"id": project["id"], "field": field["id"], "options": {o["name"]: o["id"] for o in field["options"]}}

    def item_id(self, project_id, content_id):
        node = self._query(ITEMS, id=content_id)["node"] or {}
        items = (node.get("projectItems") or {}).get("nodes", [])
        return next((item["id"] for item in items if item["project"]["id"] == project_id), None)

    def add_item(self, project_id, content_id):
        return self._query(ADD_ITEM, project=project_id, content=content_id)["addProjectV2ItemById"]["item"]["id"]

    def set_status(self, project_id, item_id, field_id, option_id):
        self._query(SET_STATUS, project=project_id, item=item_id, field=field_id, option=option_id)
