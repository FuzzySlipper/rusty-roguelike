import {
  browserHttpClient,
  type HttpClientPort,
} from '@rusty-roguelike/platform';
import {
  decodeBootstrapReadout,
  decodeSessionError,
  decodeSessionView,
  type BootstrapReadoutDto,
  type SessionCommandDto,
  type SessionView,
} from '@rusty-roguelike/protocol';

export class BootstrapTransport {
  constructor(private readonly http: HttpClientPort = browserHttpClient) {}

  async load(): Promise<BootstrapReadoutDto> {
    const response = await this.http.get('/api/v1/bootstrap');
    if (!response.ok) {
      throw new Error(`bootstrap request failed with HTTP ${response.status}`);
    }
    return decodeBootstrapReadout(await response.json());
  }
}

export class SessionTransportError extends Error {
  constructor(
    readonly code: string,
    readonly status: number,
    detail: string,
  ) {
    super(detail);
    this.name = 'SessionTransportError';
  }
}

export class SessionTransport {
  constructor(private readonly http: HttpClientPort = browserHttpClient) {}

  async load(): Promise<SessionView> {
    return this.decode(await this.http.get('/api/v1/session'));
  }

  async command(command: SessionCommandDto): Promise<SessionView> {
    return this.decode(
      await this.http.post('/api/v1/session/commands', command),
    );
  }

  async save(): Promise<SessionView> {
    return this.decode(await this.http.post('/api/v1/session/save', {}));
  }

  async reopen(): Promise<SessionView> {
    return this.decode(await this.http.post('/api/v1/session/reopen', {}));
  }

  private async decode(response: {
    readonly ok: boolean;
    readonly status: number;
    json(): Promise<unknown>;
  }): Promise<SessionView> {
    const body = await response.json();
    if (!response.ok) {
      const failure = decodeSessionError(body);
      throw new SessionTransportError(
        failure.code,
        response.status,
        failure.detail,
      );
    }
    return decodeSessionView(body);
  }
}
