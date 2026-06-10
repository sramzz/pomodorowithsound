import { describe, expect, it } from "vitest";
import { computePomodoroCount } from "./estimation";

describe("computePomodoroCount", () => {
  it("divides the estimate by the work length, rounding up", () => {
    expect(computePomodoroCount(40, 20)).toBe(2);
    expect(computePomodoroCount(45, 20)).toBe(3);
    expect(computePomodoroCount(50, 50)).toBe(1);
  });
  it("never returns less than one pomodoro", () => {
    expect(computePomodoroCount(5, 20)).toBe(1);
    expect(computePomodoroCount(0, 20)).toBe(1);
  });
  it("falls back to the spec's 20-minute work length when none is known", () => {
    expect(computePomodoroCount(60, null)).toBe(3);
  });
});
