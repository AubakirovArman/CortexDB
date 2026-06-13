import { CortexDBError } from "./errors";

export type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

export interface ClientOptions {
  timeoutMs?: number;
  fetch?: FetchLike;
}

export interface RequestJsonOptions {
  baseUrl: string;
  path: string;
  method: string;
  token?: string;
  body?: unknown;
  maxRetries: number;
  retryDelayMs: number;
  timeoutMs: number;
  fetch: FetchLike;
}

export async function requestJson(options: RequestJsonOptions): Promise<unknown> {
  const url = `${options.baseUrl}${options.path}`;
  const body = encodeBody(options.body);
  let attempt = 0;
  while (true) {
    const controller = options.timeoutMs > 0 ? new AbortController() : null;
    const timeout = controller
      ? setTimeout(() => controller.abort(), options.timeoutMs)
      : null;
    try {
      const response = await options.fetch(url, buildInit(options, body, controller));
      if (!response.ok) {
        if (attempt < options.maxRetries && await isRetryableResponse(response)) {
          attempt += 1;
          await sleep(options.retryDelayMs * attempt);
          continue;
        }
        throw await CortexDBError.fromResponse(response);
      }
      return response.json();
    } catch (error) {
      if (error instanceof CortexDBError) throw error;
      if (attempt < options.maxRetries) {
        attempt += 1;
        await sleep(options.retryDelayMs * attempt);
        continue;
      }
      throw error;
    } finally {
      if (timeout) clearTimeout(timeout);
    }
  }
}

export async function isRetryableResponse(response: Response): Promise<boolean> {
  if (response.status === 502 || response.status === 504) return true;
  if (response.status !== 503) return false;
  const code = await responseErrorCode(response);
  return code === "database_busy" || code === "service_unavailable";
}

export function scopedPath(path: string, tenant?: string): string {
  if (!tenant || tenant === "default") return path;
  const params = new URLSearchParams({ tenant });
  return `${path}${path.includes("?") ? "&" : "?"}${params.toString()}`;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function buildInit(
  options: RequestJsonOptions,
  body: string | undefined,
  controller: AbortController | null,
): RequestInit {
  const headers: Record<string, string> = {};
  if (options.token) headers.authorization = `Bearer ${options.token}`;
  if (body !== undefined) headers["content-type"] = "application/json";
  return {
    method: options.method,
    headers,
    body,
    signal: controller?.signal,
  };
}

function encodeBody(body: unknown): string | undefined {
  if (body === undefined) return undefined;
  return typeof body === "string" ? body : JSON.stringify(body);
}

async function responseErrorCode(response: Response): Promise<string | null> {
  try {
    const text = await response.clone().text();
    const data = JSON.parse(text) as { code?: unknown; error?: unknown };
    return data.code || data.error ? String(data.code ?? data.error) : null;
  } catch {
    return null;
  }
}
