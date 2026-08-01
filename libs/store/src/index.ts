import { Injectable, signal } from '@angular/core';

import type {
  BootstrapReadoutDto,
  SessionCommandDto,
  SessionLogEntry,
  SessionView,
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

export type RulesLogEntry = SessionLogEntry;

export interface SessionCommandFailure {
  readonly code: string | null;
  readonly detail: string;
}

export interface SessionTransportPort {
  load(): Promise<SessionView>;
  command(command: SessionCommandDto): Promise<SessionView>;
  save(): Promise<SessionView>;
  reopen(): Promise<SessionView>;
}

export class SessionStoreCore {
  readonly state = signal<SessionState>({ status: 'loading' });
  readonly busy = signal(false);
  readonly commandError = signal<SessionCommandFailure | null>(null);
  readonly log = signal<readonly RulesLogEntry[]>([]);
  readonly persistenceNotice = signal<string | null>(null);

  constructor(private readonly transport: SessionTransportPort) {}

  async load(): Promise<void> {
    this.state.set({ status: 'loading' });
    this.commandError.set(null);
    this.persistenceNotice.set(null);
    try {
      const value = await this.transport.load();
      this.state.set({ status: 'ready', value });
      this.publishLog(value);
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
    this.persistenceNotice.set(null);
    try {
      const value = await this.transport.command(command);
      this.state.set({ status: 'ready', value });
      this.publishLog(value);
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

  async save(): Promise<boolean> {
    return this.persistenceRequest('Session saved.', () =>
      this.transport.save(),
    );
  }

  async reopen(): Promise<boolean> {
    return this.persistenceRequest('Saved session reopened.', () =>
      this.transport.reopen(),
    );
  }

  private async persistenceRequest(
    success: string,
    request: () => Promise<SessionView>,
  ): Promise<boolean> {
    if (this.busy() || this.state().status !== 'ready') {
      return false;
    }
    this.busy.set(true);
    this.commandError.set(null);
    this.persistenceNotice.set(null);
    try {
      const value = await request();
      this.state.set({ status: 'ready', value });
      this.publishLog(value);
      this.persistenceNotice.set(success);
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

  private publishLog(value: SessionView): void {
    this.log.set(value.log);
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
