export function normalizeError(error: unknown, fallback = "操作失败，请稍后重试。") {
  const message = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  return message || fallback;
}

export function normalizeUpdateError(error: unknown, fallback: string, unavailableText: string) {
  const message = normalizeError(error, fallback);
  return !message || /not implemented|not available|permission|plugin/i.test(message)
    ? unavailableText
    : message;
}
