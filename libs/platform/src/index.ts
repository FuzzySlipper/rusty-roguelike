export interface HttpResponsePort {
  readonly ok: boolean;
  readonly status: number;
  json(): Promise<unknown>;
}

export interface HttpClientPort {
  get(url: string): Promise<HttpResponsePort>;
  post(url: string, body: unknown): Promise<HttpResponsePort>;
}

export const browserHttpClient: HttpClientPort = {
  get: async (url) => fetch(url),
  post: async (url, body) =>
    fetch(url, {
      body: JSON.stringify(body),
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    }),
};

export function browserDevicePixelRatio(): number {
  const ratio = globalThis.devicePixelRatio;
  return Number.isFinite(ratio) && ratio > 0 ? Math.min(ratio, 2) : 1;
}

export function browserPrefersReducedMotion(): boolean {
  return (
    globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
  );
}

export function requestBrowserFrame(callback: FrameRequestCallback): number {
  return globalThis.requestAnimationFrame(callback);
}

export function cancelBrowserFrame(handle: number): void {
  globalThis.cancelAnimationFrame(handle);
}

export function browserNow(): number {
  return globalThis.performance.now();
}

export function observeGlobalKeydown(
  listener: (event: KeyboardEvent) => void,
): () => void {
  globalThis.addEventListener('keydown', listener);
  return () => globalThis.removeEventListener('keydown', listener);
}

export function keyboardEventTargetsEditable(event: KeyboardEvent): boolean {
  const target = event.target;
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName))
  );
}

export function observeElementSize(
  element: Element,
  onSize: (size: { readonly width: number; readonly height: number }) => void,
): () => void {
  const observer = new ResizeObserver((entries) => {
    const entry = entries[0];
    if (entry !== undefined) {
      onSize({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      });
    }
  });
  observer.observe(element);
  return () => observer.disconnect();
}
