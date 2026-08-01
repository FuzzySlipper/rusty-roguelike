import { Injectable, signal } from '@angular/core';

import type { BootstrapReadoutDto } from '@rusty-roguelike/protocol';
import { BootstrapTransport } from '@rusty-roguelike/transport';

export type BootstrapState =
  | { readonly status: 'loading' }
  | { readonly status: 'ready'; readonly value: BootstrapReadoutDto }
  | { readonly status: 'error'; readonly message: string };

@Injectable({ providedIn: 'root' })
export class BootstrapStore {
  readonly state = signal<BootstrapState>({ status: 'loading' });
  private readonly transport = new BootstrapTransport();

  async load(): Promise<void> {
    this.state.set({ status: 'loading' });
    try {
      this.state.set({ status: 'ready', value: await this.transport.load() });
    } catch (error) {
      this.state.set({
        status: 'error',
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }
}
