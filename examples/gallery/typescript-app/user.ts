// User store built on the shared types.

import { User, Result, ok, err } from "./types";

export class UserStore {
  private users = new Map<number, User>();
  private nextId = 1;

  create(email: string): User {
    const user: User = { id: this.nextId++, email, isActive: true };
    this.users.set(user.id, user);
    return user;
  }

  find(id: number): Result<User> {
    const user = this.users.get(id);
    return user ? ok(user) : err(`no user ${id}`);
  }

  deactivate(id: number): void {
    const user = this.users.get(id);
    if (user) user.isActive = false;
  }
}
