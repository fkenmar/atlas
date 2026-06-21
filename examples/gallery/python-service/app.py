"""HTTP entry points wiring requests to the user repository."""

from __future__ import annotations

from models import User, UserRepository


class App:
    def __init__(self, users: UserRepository | None = None) -> None:
        self.users = users or UserRepository()

    def get_user(self, user_id: int) -> dict | None:
        user = self.users.get(user_id)
        return _serialize(user) if user else None

    def register(self, email: str) -> dict:
        return _serialize(self.users.create(email))


def _serialize(user: User) -> dict:
    return {"id": user.id, "email": user.email, "active": user.is_active}


def main() -> None:
    app = App()
    print(app.register("alice@example.com"))


if __name__ == "__main__":
    main()
