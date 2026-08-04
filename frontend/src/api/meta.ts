import type { BuildInfo } from './types';
import { fetchJson } from './transport';

export async function getBuildInfo(): Promise<BuildInfo> {
  return fetchJson<BuildInfo>('/api/meta/build');
}
