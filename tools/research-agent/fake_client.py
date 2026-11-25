"""A stand-in for anthropic.Anthropic() that plays back scripted responses, offline.

Blocks and responses mimic the SDK's attribute shape (.type, .text, .id, .name, .input,
.stop_reason, .content), which is all agent.py reads. Each request is deep-copied when
it arrives, so tests see exactly what Claude would have been sent on that call.
"""

import copy
from types import SimpleNamespace


def text(t: str):
    return SimpleNamespace(type="text", text=t)


def tool_use(block_id: str, name: str, tool_input: dict):
    return SimpleNamespace(type="tool_use", id=block_id, name=name, input=tool_input)


def response(stop_reason: str, *blocks):
    return SimpleNamespace(stop_reason=stop_reason, content=list(blocks))


class FakeClient:
    def __init__(self, responses):
        self._responses = list(responses)
        self.calls = []
        self.messages = self  # so client.messages.create(...) works

    def create(self, **request):
        self.calls.append(copy.deepcopy(request))
        if not self._responses:
            raise AssertionError("the agent called Claude more times than the script allows")
        return self._responses.pop(0)


class RoutedFakeClient:
    """One script per agent, for runs where a coordinator and its readers share a client.

    route(request) names whose turn it is (e.g. "coordinator", "reader:14"); each name
    plays back its own list of responses, so the order across agents doesn't matter.
    """

    def __init__(self, route, scripts: dict):
        self._route = route
        self._scripts = {name: list(responses) for name, responses in scripts.items()}
        self.calls = []  # (name, request)
        self.messages = self

    def create(self, **request):
        name = self._route(request)
        self.calls.append((name, copy.deepcopy(request)))
        if not self._scripts.get(name):
            raise AssertionError(f"no scripted response left for {name}")
        return self._scripts[name].pop(0)

    def calls_for(self, name: str) -> list:
        return [request for who, request in self.calls if who == name]
