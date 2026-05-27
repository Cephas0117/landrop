export type ToastKind = "success" | "error" | "info" | "warn";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  detail?: string;
}

let _toasts = $state<Toast[]>([]);
let _seq = 0;

export const toasts = {
  get list() {
    return _toasts;
  },

  push(kind: ToastKind, message: string, detail?: string, ttl = 4000) {
    const id = ++_seq;
    _toasts = [..._toasts, { id, kind, message, detail }];
    setTimeout(() => {
      _toasts = _toasts.filter((t) => t.id !== id);
    }, ttl);
  },

  success(msg: string, detail?: string) { this.push("success", msg, detail); },
  error(msg: string, detail?: string)   { this.push("error",   msg, detail, 6000); },
  info(msg: string, detail?: string)    { this.push("info",    msg, detail); },
  warn(msg: string, detail?: string)    { this.push("warn",    msg, detail, 5000); },

  dismiss(id: number) {
    _toasts = _toasts.filter((t) => t.id !== id);
  },
};
