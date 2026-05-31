(() => {
    const reports = window.CortexDashboardReports || {};

    function q16Percent(value) {
        if (typeof value !== "number") return "n/a";
        return `${((value * 100) / 65535).toFixed(2)}%`;
    }

    function yesNo(value) {
        if (value === true) return "yes";
        if (value === false) return "no";
        return "n/a";
    }

    function card(label, value, tone = "") {
        const node = document.createElement("div");
        const labelNode = document.createElement("span");
        const valueNode = document.createElement("strong");
        node.className = tone ? `metric ${tone}` : "metric";
        labelNode.textContent = label;
        valueNode.textContent = String(value);
        node.append(labelNode, valueNode);
        return node;
    }

    function sourceLabel(sourceRef) {
        if (!sourceRef) return "";
        const parts = [sourceRef.source_id, sourceRef.document_id].filter(Boolean);
        if (sourceRef.page !== null && sourceRef.page !== undefined) parts.push(`page ${sourceRef.page}`);
        if (sourceRef.json_path) parts.push(sourceRef.json_path);
        return parts.join(" · ");
    }

    function preview(text) {
        if (!text) return "no payload preview";
        const compact = String(text).replace(/\s+/g, " ").trim();
        return compact.length > 180 ? `${compact.slice(0, 177)}...` : compact;
    }

    function textItem(text) {
        const item = document.createElement("li");
        item.textContent = text;
        return item;
    }

    reports.helpers = {
        card,
        preview,
        q16Percent,
        sourceLabel,
        textItem,
        yesNo,
    };
    reports.version = "dashboard-reports.v1";
    window.CortexDashboardReports = reports;
})();
