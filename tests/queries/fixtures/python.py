"""Fixture for queries/python/tags.scm — one construct per extraction rule."""

import os
from typing import Optional

API_VERSION = "1.0"


def top_level(x: int) -> int:
    return helper(x)


async def fetch(url: str) -> Optional[str]:
    return os.environ.get(url)


def helper(x: int) -> int:
    return x + 1


class Service:
    """A class with plain, property, and decorated methods."""

    @property
    def name(self) -> str:
        return self._name

    def run(self) -> None:
        self.helper_method()

    @staticmethod
    def helper_method() -> None:
        pass


class Mark:
    """A dataclass-style class with annotated fields (PRD §5.3)."""

    name: str
    args: tuple
    description: str | None = None
    _internal: int = 0


def decorating(fn):
    return fn


@decorating
def decorated() -> None:
    pass


@decorating
class DecoratedService:
    pass
