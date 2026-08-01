import { Injectable, signal } from '@angular/core';

import type {
  BootstrapReadoutDto,
  SessionCommandDto,
  SessionView,
  TurnReceipt,
} from '@rusty-roguelike/protocol';
import {
  BootstrapTransport,
  SessionTransport,
  SessionTransportError,
} from '@rusty-roguelike/transport';

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

export type SessionState =
  | { readonly status: 'loading' }
  | { readonly status: 'ready'; readonly value: SessionView }
  | { readonly status: 'error'; readonly message: string };

export interface RulesLogEntry {
  readonly id: string;
  readonly revision: number;
  readonly receipt: TurnReceipt;
}

export interface SessionCommandFailure {
  readonly code: string | null;
  readonly detail: string;
}

export interface SessionTransportPort {
  load(): Promise<SessionView>;
  command(command: SessionCommandDto): Promise<SessionView>;
}

export class SessionStoreCore {
  readonly state = signal<SessionState>({ status: 'loading' });
  readonly busy = signal(false);
  readonly commandError = signal<SessionCommandFailure | null>(null);
  readonly log = signal<readonly RulesLogEntry[]>([]);

  constructor(private readonly transport: SessionTransportPort) {}

  async load(): Promise<void> {
    this.state.set({ status: 'loading' });
    this.commandError.set(null);
    try {
      const value = await this.transport.load();
      this.state.set({ status: 'ready', value });
      this.publishReceipts(value);
    } catch (error) {
      this.state.set({ status: 'error', message: message(error) });
    }
  }

  async command(command: SessionCommandDto): Promise<boolean> {
    if (this.busy() || this.state().status !== 'ready') {
      return false;
    }
    this.busy.set(true);
    this.commandError.set(null);
    try {
      const value = await this.transport.command(command);
      this.state.set({ status: 'ready', value });
      this.publishReceipts(value);
      return true;
    } catch (error) {
      this.commandError.set(
        error instanceof SessionTransportError
          ? { code: error.code, detail: error.message }
          : { code: null, detail: message(error) },
      );
      return false;
    } finally {
      this.busy.set(false);
    }
  }

  private publishReceipts(value: SessionView): void {
    if (value.latestReceipts.length === 0) {
      return;
    }
    const known = new Set(this.log().map((entry) => entry.id));
    const appended = value.latestReceipts
      .map((receipt, index) => ({
        id: `${value.revision}.${index}`,
        receipt,
        revision: value.revision,
      }))
      .filter((entry) => !known.has(entry.id));
    this.log.update((entries) => [...entries, ...appended].slice(-128));
  }
}

@Injectable({ providedIn: 'root' })
export class SessionStore extends SessionStoreCore {
  constructor() {
    super(new SessionTransport());
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
