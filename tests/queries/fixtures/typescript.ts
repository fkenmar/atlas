// Fixture for queries/typescript/tags.scm — one construct per extraction rule.

import { join } from "node:path";

export const API_VERSION: string = "1.0";

export function topLevel(x: number): number {
  return helper(x);
}

const arrow = (x: number): number => x * 2;

function helper(x: number): number {
  return arrow(x) + 1;
}

export interface Options {
  budget: number;
}

export type Format = "md" | "json" | "xml";

export enum Level {
  Low,
  High,
}

export class Service {
  run(): void {
    this.helperMethod();
  }

  private helperMethod(): void {
    void join("a", "b");
  }
}

// Overload signatures + implementation: same declaration node kinds.
export function overloaded(x: number): number;
export function overloaded(x: string): string;
export function overloaded(x: number | string): number | string {
  return x;
}

// Ambient declaration.
declare function ambient(x: number): void;
