"""Domain models backed by the database layer."""

from __future__ import annotations

from dataclasses import dataclass

from db import Database, connect


@dataclass
class User:
    id: int
    email: str
    is_active: bool = True


class UserRepository:
    """Loads and stores `User` records."""

    def __init__(self, db: Database | None = None) -> None:
        self._db = db or connect()

    def get(self, user_id: int) -> User | None:
        row = self._db.query_one(
            "SELECT id, email, is_active FROM users WHERE id = ?", (user_id,)
        )
        if row is None:
            return None
        return User(id=row[0], email=row[1], is_active=bool(row[2]))

    def create(self, email: str) -> User:
        cursor = self._db.execute(
            "INSERT INTO users (email, is_active) VALUES (?, 1)", (email,)
        )
        self._db.commit()
        return User(id=cursor.lastrowid, email=email)
