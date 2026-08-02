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

export async function loadBrowserBinaryAsset(
  url: string,
): Promise<ArrayBuffer> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`asset request failed with HTTP ${response.status}`);
  }
  return response.arrayBuffer();
}

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

export function keyboardEventTargetsInteractive(event: KeyboardEvent): boolean {
  const target = event.target;
  return (
    target instanceof Element &&
    target.closest(
      'button, a, summary, details, [role="button"], [role="menuitem"], [role="option"]',
    ) !== null
  );
}

const ROGUELIKE_ITEM_DRAG_TYPE = 'application/x-rusty-roguelike-loadout-item';

export interface LoadoutDragPayload {
  readonly itemEntityId: number;
  readonly ownerEntityId: number;
}

export function writeLoadoutDrag(
  transfer: DataTransfer | null,
  payload: LoadoutDragPayload,
): boolean {
  if (transfer === null) {
    return false;
  }
  transfer.effectAllowed = 'move';
  transfer.setData(
    ROGUELIKE_ITEM_DRAG_TYPE,
    `${payload.itemEntityId}:${payload.ownerEntityId}`,
  );
  return true;
}

export function admitLoadoutDrag(transfer: DataTransfer | null): boolean {
  return transfer?.types.includes(ROGUELIKE_ITEM_DRAG_TYPE) ?? false;
}

export function markLoadoutDragMove(transfer: DataTransfer | null): void {
  if (transfer !== null) {
    transfer.dropEffect = 'move';
  }
}

export function readLoadoutDrag(
  transfer: DataTransfer | null,
): LoadoutDragPayload | null {
  const encoded = transfer?.getData(ROGUELIKE_ITEM_DRAG_TYPE) ?? '';
  const match = /^(\d{1,16}):(\d{1,16})$/.exec(encoded);
  if (match === null) {
    return null;
  }
  const itemEntityId = Number(match[1]);
  const ownerEntityId = Number(match[2]);
  return Number.isSafeInteger(itemEntityId) &&
    itemEntityId > 0 &&
    Number.isSafeInteger(ownerEntityId) &&
    ownerEntityId > 0
    ? { itemEntityId, ownerEntityId }
    : null;
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
