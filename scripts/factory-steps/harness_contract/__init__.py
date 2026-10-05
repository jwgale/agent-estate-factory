"""Cell One harness job contract. Additive. Does not edit factory_steps.py.

Grok Bot drops a job file. This package writes status, a card, and a reading
page. It does not chat, schedule, notify, wrap connectors, or start a train.
"""

from .runner import Harness

__all__ = ["Harness"]
