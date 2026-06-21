// HTTP-ish handlers over the user store.

import { UserStore } from "./user";
import { Result, User } from "./types";

export class Api {
  constructor(private store: UserStore = new UserStore()) {}

  register(email: string): User {
    return this.store.create(email);
  }

  lookup(id: number): Result<User> {
    return this.store.find(id);
  }
}

export function createApi(): Api {
  return new Api();
}
