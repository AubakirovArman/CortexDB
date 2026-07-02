pub(crate) fn integer_sqrt(val: u128) -> u128 {
    let mut res = 0;
    let mut add = 1u128 << 63;
    while add > 0 {
        let temp = res + add;
        if temp * temp <= val {
            res = temp;
        }
        add >>= 1;
    }
    res
}

pub(crate) fn cosine_similarity_q16(u: &[i16], v: &[i16]) -> u16 {
    if u.len() != v.len() || u.is_empty() {
        return 0;
    }

    let mut dot_product: i128 = 0;
    let mut norm_u_sq: u128 = 0;
    let mut norm_v_sq: u128 = 0;
    for (left, right) in u.iter().zip(v) {
        let left = i128::from(*left);
        let right = i128::from(*right);
        dot_product += left * right;
        norm_u_sq += (left * left) as u128;
        norm_v_sq += (right * right) as u128;
    }

    if norm_u_sq == 0 || norm_v_sq == 0 || dot_product <= 0 {
        return 0;
    }

    let norm_u = integer_sqrt(norm_u_sq);
    let norm_v = integer_sqrt(norm_v_sq);
    if norm_u == 0 || norm_v == 0 {
        return 0;
    }

    let norm_product = norm_u.saturating_mul(norm_v);
    if norm_product == 0 {
        return 0;
    }

    let dot_scaled = (dot_product as u128).saturating_mul(u128::from(u16::MAX) + 1);
    (dot_scaled / norm_product).min(u128::from(u16::MAX)) as u16
}
