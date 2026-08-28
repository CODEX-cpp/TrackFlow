import { AWClient } from 'aw-client';
import type { AxiosInstance } from 'axios';

import { useSettingsStore } from '~/stores/settings';
// Log diagnostico avanzato, disattivato di default — vedi util/diagnostics.ts.
import { strumentaClient } from '~/util/diagnostics';

const API_TOKEN_QUERY_PARAM = 'token';
const API_TOKEN_STORAGE_KEY = 'aw-api-token';

let _client: AWClient | null;

function normalizeToken(token: string | null): string | null {
  if (!token) {
    return null;
  }

  const trimmed = token.trim();
  return trimmed ? trimmed : null;
}

function getSessionStorage(): Storage | null {
  if (typeof sessionStorage === 'undefined') {
    return null;
  }

  return sessionStorage;
}

export function getStoredApiToken(): string | null {
  return normalizeToken(getSessionStorage()?.getItem(API_TOKEN_STORAGE_KEY) ?? null);
}

export function getApiTokenFromLocation(currentLocation: Pick<Location, 'search'>): string | null {
  if (typeof URLSearchParams === 'undefined') {
    return null;
  }

  return normalizeToken(new URLSearchParams(currentLocation.search).get(API_TOKEN_QUERY_PARAM));
}

function persistApiToken(token: string): void {
  getSessionStorage()?.setItem(API_TOKEN_STORAGE_KEY, token);
}

export function stripApiTokenFromCurrentUrl(): void {
  if (typeof window === 'undefined' || typeof history === 'undefined') {
    return;
  }

  const url = new URL(window.location.href);
  if (!url.searchParams.has(API_TOKEN_QUERY_PARAM)) {
    return;
  }

  url.searchParams.delete(API_TOKEN_QUERY_PARAM);
  const nextUrl = `${url.pathname}${url.search}${url.hash}`;
  window.history.replaceState(window.history.state, '', nextUrl || '/');
}

// NOTE: The token is visible in window.location.href from page load until this
// function executes.  In the intended WebView/launcher environment no third-party
// scripts run, so the exposure window is acceptable.  For general browser use,
// consider passing the credential via the URL fragment instead of a query param.
export function loadApiTokenFromBrowser(): string | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const urlToken = getApiTokenFromLocation(window.location);
  if (urlToken) {
    persistApiToken(urlToken);
    stripApiTokenFromCurrentUrl();
    return urlToken;
  }

  return getStoredApiToken();
}

export function applyApiToken(client: { req: AxiosInstance }, token: string | null): void {
  const defaults = client.req.defaults;
  const headers = (defaults.headers as Record<string, Record<string, unknown>>) || {};
  const commonHeaders = headers.common || {};

  if (token) {
    commonHeaders.Authorization = `Bearer ${token}`;
  } else {
    delete commonHeaders.Authorization;
  }

  headers.common = commonHeaders;
  defaults.headers = headers as typeof defaults.headers;
}

function resolveBaseURL(): string {
  const production = typeof PRODUCTION !== 'undefined' && PRODUCTION;
  // If running with `npm node dev`, use testing server as origin.
  // Works since CORS is enabled by default when running `aw-server --testing`.
  if (production) return '';
  const aw_server_url = typeof AW_SERVER_URL !== 'undefined' && AW_SERVER_URL;
  return aw_server_url || 'http://127.0.0.1:5666';
}

export function createClient(force?: boolean): AWClient {
  const production = typeof PRODUCTION !== 'undefined' && PRODUCTION;

  if (!_client || force) {
    _client = new AWClient('aw-webui', {
      testing: !production,
      baseURL: resolveBaseURL(),
    });
    applyApiToken(_client, loadApiTokenFromBrowser());
    strumentaClient(_client, 'globale');
  } else {
    throw 'Tried to instantiate global AWClient twice!';
  }
  return _client;
}

// A second AWClient instance, entirely separate from the shared global
// one (`getClient()`), each with its own AbortController. The global
// client is aborted wholesale by activityStore.ensure_loaded() (see
// stores/activity.ts) whenever the old Activity page's date/period
// changes — a reasonable way for THAT store to cancel its own stale
// in-flight queries, but since aw-client's abort() cancels every
// request sharing its controller, it collaterally cancelled unrelated
// requests from the new Home page (Timeline, VPN/Claude summary
// panels) whenever their fetch happened to be in flight at the same
// moment — surfacing as an empty Timeline after switching days while
// the summary panels (which go through a different query path) kept
// working. New Home-page code that reads raw bucket events directly
// (not through activityStore) should use this client instead of $aw.
let _homeClient: AWClient | null = null;

export function getHomeClient(): AWClient {
  if (!_homeClient) {
    const production = typeof PRODUCTION !== 'undefined' && PRODUCTION;
    _homeClient = new AWClient('aw-webui-home', {
      testing: !production,
      baseURL: resolveBaseURL(),
    });
    applyApiToken(_homeClient, loadApiTokenFromBrowser());
    strumentaClient(_homeClient, 'home');
  }
  return _homeClient;
}

export function configureClient(): void {
  const settings = useSettingsStore();
  _client.req.defaults.timeout = 1000 * settings.requestTimeout;
}

export function getClient(): AWClient {
  if (!_client) {
    throw 'Tried to get global AWClient before instantiating it!';
  }
  return _client;
}
