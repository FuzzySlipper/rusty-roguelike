export interface HttpResponsePort {
  readonly ok: boolean;
  readonly status: number;
  json(): Promise<unknown>;
}

export interface HttpClientPort {
  get(url: string): Promise<HttpResponsePort>;
}

export const browserHttpClient: HttpClientPort = {
  get: async (url) => fetch(url),
};

export function browserDevicePixelRatio(): number {
  const ratio = globalThis.devicePixelRatio;
  return Number.isFinite(ratio) && ratio > 0 ? Math.min(ratio, 2) : 1;
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
