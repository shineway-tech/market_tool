import type { StartLoginResponse } from "./types";

export function isQrAuth(task: StartLoginResponse) {
  return task.authType === "qrcode" || task.url.startsWith("data:image");
}
