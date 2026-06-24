import type { ApiResponse } from "../domain/types";

export type ApiMethod = "GET" | "POST" | "PUT" | "DELETE";

export interface ApiRequestOptions {
  method?: ApiMethod;
  body?: Record<string, unknown>;
  skipAuth?: boolean;
  token?: string;
  onUnauthorized?: () => void;
}

export async function requestApi<T>(
  baseUrl: string,
  path: string,
  {
    method = "GET",
    body,
    skipAuth = false,
    token = "",
    onUnauthorized,
  }: ApiRequestOptions = {},
) {
  const headers: Record<string, string> = {};

  if (token && !skipAuth) {
    headers["X-Token"] = token;
  }

  if (body) {
    headers["Content-Type"] = "application/json";
  }

  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  const payload = await response.json().catch(() => null) as ApiResponse<T> | null;

  if (!response.ok || !payload || payload.err_code !== 0) {
    if (response.status === 401) {
      onUnauthorized?.();
    }
    throw new Error(payload?.err_msg || `HTTP ${response.status}`);
  }

  return payload.data;
}
