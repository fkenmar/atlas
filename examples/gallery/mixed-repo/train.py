"""Offline weight-training script (Python)."""

from __future__ import annotations

from typing import Sequence


def gradient_step(weights: list[float], features: Sequence[float], target: float, lr: float = 0.01) -> list[float]:
    """One SGD update of `weights` toward `target`."""
    pred = sum(w * f for w, f in zip(weights, features))
    error = pred - target
    return [w - lr * error * f for w, f in zip(weights, features)]


def train(rows: list[tuple[list[float], float]], epochs: int = 10) -> list[float]:
    weights = [0.0, 0.0, 0.0]
    for _ in range(epochs):
        for features, target in rows:
            weights = gradient_step(weights, features, target)
    return weights


if __name__ == "__main__":
    print(train([([1.0, 2.0, 3.0], 1.0)]))
