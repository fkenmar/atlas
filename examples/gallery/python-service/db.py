"""Database connection and query helpers."""

from __future__ import annotations

import sqlite3
from typing import Any, Iterable


class Database:
    """Thin wrapper around a SQLite connection."""

    def __init__(self, path: str) -> None:
        self._conn = sqlite3.connect(path)

    def execute(self, sql: str, params: Iterable[Any] = ()) -> sqlite3.Cursor:
        return self._conn.execute(sql, tuple(params))

    def query_one(self, sql: str, params: Iterable[Any] = ()) -> tuple | None:
        return self.execute(sql, params).fetchone()

    def commit(self) -> None:
        self._conn.commit()

    def close(self) -> None:
        self._conn.close()


def connect(path: str = "app.db") -> Database:
    """Open the application database."""
    return Database(path)
