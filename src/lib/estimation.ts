/** Spec §2: if no pomodoro type resolves, fall back to 20 minutes of work. */
const FALLBACK_WORK_MINUTES = 20;

export function computePomodoroCount(
  estimatedMinutes: number,
  workMinutes: number | null,
): number {
  const work = workMinutes && workMinutes > 0 ? workMinutes : FALLBACK_WORK_MINUTES;
  if (estimatedMinutes <= 0) return 1;
  return Math.max(1, Math.ceil(estimatedMinutes / work));
}
