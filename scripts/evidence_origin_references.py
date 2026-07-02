"""Classify local/generated references inside operator evidence JSON."""

from __future__ import annotations

from ipaddress import IPv4Address, ip_address
from urllib.parse import unquote, urlsplit


TEMPORARY_LOCAL_PATHS = ("/tmp", "/var/tmp", "/dev/shm")
LOCAL_REFERENCE_SEGMENTS = {"fixtures", "target"}
LOCAL_REFERENCE_HOSTS = {"localhost", "0.0.0.0", "::1"}
LOCAL_TRANSPORT_SCHEMES = ("unix:", "npipe:", "pipe:")
SHELL_LOCAL_PATH_PREFIXES = (
    "$home",
    "${home}",
    "$userprofile",
    "${userprofile}",
    "$homedrive",
    "${homedrive}",
    "$homepath",
    "${homepath}",
    "$tmpdir",
    "${tmpdir}",
    "$temp",
    "${temp}",
    "$tmp",
    "${tmp}",
    "%userprofile%",
    "%homedrive%",
    "%homepath%",
    "%temp%",
    "%tmp%",
)
MAX_REFERENCE_DECODE_PASSES = 5


def is_temporary_local_path(normalized_path: str) -> bool:
    return any(
        normalized_path == prefix or normalized_path.startswith(prefix + "/")
        for prefix in TEMPORARY_LOCAL_PATHS
    )


def is_local_reference(value: str) -> bool:
    normalized = normalized_reference_value(value)
    if normalized.startswith("file:"):
        return True
    if is_loopback_reference(normalized):
        return True
    if has_local_transport_scheme(normalized):
        return True
    if is_shell_local_path_reference(normalized):
        return True
    if has_non_file_uri_scheme(normalized):
        return False
    if is_windows_absolute_path(normalized):
        return True
    if is_unc_or_scheme_relative_path(normalized):
        return True
    if is_temporary_local_path(normalized):
        return True
    if is_path_reference(normalized):
        return True
    segments = {part for part in normalized.split("/") if part not in {"", ".", ".."}}
    return bool(LOCAL_REFERENCE_SEGMENTS.intersection(segments))


def normalized_reference_value(value: str) -> str:
    normalized = value.strip().replace("\\", "/")
    for _ in range(MAX_REFERENCE_DECODE_PASSES):
        decoded = unquote(normalized).replace("\\", "/")
        if decoded == normalized:
            break
        normalized = decoded
    return normalized.lower()


def is_loopback_reference(value: str) -> bool:
    parsed = urlsplit(value)
    hostname = parsed.hostname
    if hostname is None and "://" not in value:
        hostname = host_from_schemeless_reference(value)
    return is_local_reference_host(hostname)


def host_from_schemeless_reference(value: str) -> str | None:
    head = value.split("/", 1)[0]
    if not head:
        return None
    if head.startswith("[") and "]" in head:
        return head[1 : head.find("]")]
    if ":" in head:
        return head.rsplit(":", 1)[0]
    return head


def is_local_reference_host(hostname: str | None) -> bool:
    if not hostname:
        return False
    normalized = hostname.strip("[]").rstrip(".").lower()
    if normalized in LOCAL_REFERENCE_HOSTS or normalized == "0" or normalized.startswith("127."):
        return True
    try:
        address = ip_address(normalized)
    except ValueError:
        address = legacy_ipv4_address_from_host(normalized)
        if address is None:
            return False
    if address.is_loopback or address.is_unspecified:
        return True
    mapped_address = getattr(address, "ipv4_mapped", None)
    return bool(
        mapped_address
        and (mapped_address.is_loopback or mapped_address.is_unspecified)
    )


def legacy_ipv4_address_from_host(hostname: str) -> IPv4Address | None:
    parts = hostname.split(".")
    if not parts or len(parts) > 4:
        return None
    values: list[int] = []
    for part in parts:
        value = legacy_ipv4_component(part)
        if value is None:
            return None
        values.append(value)
    try:
        if len(values) == 1:
            if values[0] > 0xFFFFFFFF:
                return None
            return IPv4Address(values[0])
        if len(values) == 2:
            if values[0] > 0xFF or values[1] > 0xFFFFFF:
                return None
            return IPv4Address((values[0] << 24) | values[1])
        if len(values) == 3:
            if values[0] > 0xFF or values[1] > 0xFF or values[2] > 0xFFFF:
                return None
            return IPv4Address((values[0] << 24) | (values[1] << 16) | values[2])
        if any(value > 0xFF for value in values):
            return None
        return IPv4Address(
            (values[0] << 24) | (values[1] << 16) | (values[2] << 8) | values[3]
        )
    except ValueError:
        return None


def legacy_ipv4_component(part: str) -> int | None:
    if not part:
        return None
    lowered = part.lower()
    if lowered.startswith("0x"):
        digits = lowered[2:]
        if not digits or any(
            character not in "0123456789abcdef" for character in digits
        ):
            return None
        return int(digits, 16)
    if len(lowered) > 1 and lowered.startswith("0"):
        if any(character not in "01234567" for character in lowered):
            return None
        return int(lowered, 8)
    if not lowered.isdecimal():
        return None
    return int(lowered, 10)


def is_windows_absolute_path(value: str) -> bool:
    return len(value) >= 3 and value[0].isalpha() and value[1:3] == ":/"


def is_unc_or_scheme_relative_path(value: str) -> bool:
    return value.startswith("//")


def has_local_transport_scheme(value: str) -> bool:
    return value.startswith(LOCAL_TRANSPORT_SCHEMES)


def is_shell_local_path_reference(value: str) -> bool:
    if value == "~" or value.startswith("~/"):
        return True
    if value.startswith("~") and "/" in value:
        return True
    return any(
        value == prefix or value.startswith(prefix + "/")
        for prefix in SHELL_LOCAL_PATH_PREFIXES
    )


def is_path_reference(value: str) -> bool:
    if "/" not in value:
        return False
    if any(character.isspace() for character in value):
        return False
    return True


def has_non_file_uri_scheme(value: str) -> bool:
    scheme_end = value.find(":")
    if scheme_end <= 0:
        return False
    scheme = value[:scheme_end]
    if len(scheme) == 1 and scheme.isalpha():
        return False
    return all(character.isalnum() or character in "+-." for character in scheme)
