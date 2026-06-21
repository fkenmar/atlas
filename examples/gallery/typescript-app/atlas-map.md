# atlas: typescript-app (61 LOC, 3 files) | budget 400 | rendered 241 tok

## types.ts (#1 — imported by 2 file(s))
export interface User
    id: number
    email: string
    isActive: boolean
export type Result<T> = { ok: true; value: T } | { ok: false; error: string }
export function ok<T>(value: T): Result<T>
export function err<T>(error: string): Result<T>
used by: api.ts, user.ts

## user.ts (#2 — imported by 1 file(s))
export class UserStore
    private users = new Map<number, User>()
    private nextId = 1
    create(email: string): User
    find(id: number): Result<User>
    deactivate(id: number): void
imports: types.ts
used by: api.ts

## api.ts (#3)
export class Api
    constructor(private store: UserStore = new UserStore()) {}
    register(email: string): User
    lookup(id: number): Result<User>
export function createApi(): Api
imports: types.ts, user.ts
