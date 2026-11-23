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
