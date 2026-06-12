pub(super) fn resolve_path(path: &str, tenant: Option<&str>) -> std::path::PathBuf {
    let base = std::path::PathBuf::from(path);
    match tenant {
        Some(t) if t != "default" => base.join("realms").join(t),
        _ => base,
    }
}

pub(super) fn resolve_backup_passphrase(
    passphrase: Option<String>,
    passphrase_env: Option<String>,
) -> Result<String, String> {
    if let Some(value) = passphrase {
        return Ok(value);
    }
    let env_name = passphrase_env.unwrap_or_else(|| "CORTEXDB_BACKUP_PASSPHRASE".to_owned());
    std::env::var(&env_name).map_err(|_| {
        format!(
            "encrypted backup passphrase is required; set {env_name} or pass --passphrase-env <VAR>"
        )
    })
}
