use super::BindError;

pub(super) fn validate_iso_date(value: &str) -> Result<(), BindError> {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(BindError::InvalidDate);
    };
    let year = parse_date_part(year, 4)?;
    let month = parse_date_part(month, 2)?;
    let day = parse_date_part(day, 2)?;
    if !(1900..=2199).contains(&year) || !(1..=12).contains(&month) {
        return Err(BindError::InvalidDate);
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return Err(BindError::InvalidDate),
    };
    if day == 0 || day > max_day {
        return Err(BindError::InvalidDate);
    }
    Ok(())
}

fn parse_date_part(value: &str, expected_len: usize) -> Result<u16, BindError> {
    if value.len() != expected_len || !value.chars().all(|character| character.is_ascii_digit()) {
        return Err(BindError::InvalidDate);
    }
    value.parse::<u16>().map_err(|_| BindError::InvalidDate)
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
