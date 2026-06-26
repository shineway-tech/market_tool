export function formValue(form: HTMLFormElement, fieldName: string) {
  return String(new FormData(form).get(fieldName) || "");
}

export function reportFieldError(field: Element | RadioNodeList | null, message: string) {
  if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
    field.setCustomValidity(message);
    field.reportValidity();
  }
}

export function clearFieldError(field: HTMLInputElement | HTMLTextAreaElement) {
  field.setCustomValidity("");
}

export function clearFormFieldErrors(form: HTMLFormElement) {
  form.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea").forEach(clearFieldError);
}

export function reportNamedFieldError(form: HTMLFormElement | null, fieldName: string, message: string) {
  if (!form) return;
  reportFieldError(form.elements.namedItem(fieldName), message);
}
