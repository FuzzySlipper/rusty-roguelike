import {
  browserHttpClient,
  type HttpClientPort,
} from '@rusty-roguelike/platform';
import {
  decodeBootstrapReadout,
  type BootstrapReadoutDto,
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
