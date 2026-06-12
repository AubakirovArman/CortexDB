async function fetchJsonLike(url, init = {}) {
    const response = await fetch(url, { ...init, headers: { ...headers(), ...(init.headers || {}) } });
    const text = await response.text();
    let body;
    try {
        body = JSON.parse(text);
    } catch {
        body = text;
    }
    return { response, body, text };
}

async function api(path, init = {}) {
    const { response, body } = await fetchJsonLike(scoped(path), {
        ...init,
        headers: { ...headers(), ...(init.headers || {}) },
    });
    if (!response.ok) throw requestError(response, body);
    return body;
}

function requestError(response, body) {
    if (body && typeof body === "object" && !Array.isArray(body)) {
        return {
            ...body,
            http_status: response.status,
            http_status_text: response.statusText || "",
        };
    }
    return {
        http_status: response.status,
        http_status_text: response.statusText || "",
        message: String(body || response.statusText || "request failed"),
    };
}
