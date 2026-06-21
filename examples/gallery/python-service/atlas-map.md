# atlas: python-service (96 LOC, 3 files) | budget 400 | rendered 300 tok

## db.py (#1 — imported by 1 file(s))
class Database
    def __init__(self, path: str) -> None
    def execute(self, sql: str, params: Iterable[Any] = ()) -> sqlite3.Cursor
    def query_one(self, sql: str, params: Iterable[Any] = ()) -> tuple | None
    def commit(self) -> None
    def close(self) -> None
def connect(path: str = "app.db") -> Database
used by: models.py

## models.py (#2 — imported by 1 file(s))
class User
    id: int
    email: str
    is_active: bool = True
class UserRepository
    def __init__(self, db: Database | None = None) -> None
    def get(self, user_id: int) -> User | None
    def create(self, email: str) -> User
imports: db.py
used by: app.py

## app.py (#3)
class App
    def __init__(self, users: UserRepository | None = None) -> None
    def get_user(self, user_id: int) -> dict | None
    def register(self, email: str) -> dict
def _serialize(user: User) -> dict
def main() -> None
imports: models.py
