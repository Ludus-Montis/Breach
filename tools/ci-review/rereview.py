"""Turns a review into what CI posts. Only new findings become comments; an earlier
finding that still stands is counted, not posted again (task statement 3.6)."""


def plan(prior: list[dict], review: dict) -> dict:
    """`prior`: the last review's findings, each with an `id`. Returns what to post and the tally."""
    prior_ids = {p["id"] for p in prior}
    post, still_open = [], []
    for f in review["findings"]:
        cites_known = f["prior_id"] in prior_ids
        # A prior_id that matches no earlier finding is the model's mistake: treat the
        # finding as new rather than drop it, so a real issue is never lost.
        # STUDY 3.6: new = cites no earlier finding we know of (null, or an id we never issued).
        is_new = not cites_known
        if is_new:
            post.append(f)
        else:
            still_open.append(f["prior_id"])
    resolved = [i for i in review["resolved_prior_ids"] if i in prior_ids]
    # Earlier findings the review neither fixed nor repeated: not confirmed fixed, so CI says so.
    unmentioned = sorted(prior_ids - set(resolved) - set(still_open))
    return {"post": post, "still_open": sorted(set(still_open)), "resolved": sorted(resolved),
            "unmentioned": unmentioned}
