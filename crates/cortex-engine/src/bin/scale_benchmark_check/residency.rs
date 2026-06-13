use cortex_engine::PayloadResidency;

pub(crate) fn parse(value: String) -> Result<PayloadResidency, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "memory" => Ok(PayloadResidency::Memory),
        "lazy" => Ok(PayloadResidency::Lazy),
        _ => Err(format!(
            "invalid value for --payload-residency: {value}; expected memory or lazy"
        )),
    }
}
