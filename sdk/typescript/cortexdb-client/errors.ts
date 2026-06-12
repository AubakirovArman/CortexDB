export type JsonObject = Record<string, unknown>;

export class CortexDBError extends Error {
  constructor(
    message: string,
    public readonly code: string | null = null,
    public readonly status: number | null = null,
    public readonly body: string | null = null,
  ) {
    super(message);
    this.name = "CortexDBError";
  }

  static async fromResponse(response: Response): Promise<CortexDBError> {
    const body = await response.text();
    try {
      const data = JSON.parse(body) as JsonObject;
      return new CortexDBError(
        String(data.message ?? body),
        data.code ? String(data.code) : null,
        response.status,
        body,
      );
    } catch {
      return new CortexDBError(body, null, response.status, body);
    }
  }
}
