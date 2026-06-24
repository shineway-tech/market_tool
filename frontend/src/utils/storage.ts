export function readStoredMode<T extends string>(key: string, fallback: T, allowed: readonly T[]): T {
  const value = localStorage.getItem(key) as T | null;
  return value && allowed.includes(value) ? value : fallback;
}

export function readStoredBoolean(key: string, fallback: boolean) {
  const value = localStorage.getItem(key);
  if (value === "true") return true;
  if (value === "false") return false;
  return fallback;
}
