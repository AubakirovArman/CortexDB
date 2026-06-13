use std::collections::BTreeSet;

pub fn mutate_bytes(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut cases = Vec::new();
    for cut in unique_positions(bytes.len()) {
        cases.push((format!("truncate_{cut}"), bytes[..cut].to_vec()));
    }
    for position in unique_positions(bytes.len()) {
        if position < bytes.len() {
            let mut flipped = bytes.to_vec();
            flipped[position] ^= 0xff;
            cases.push((format!("flip_{position}"), flipped));
        }
    }
    let mut appended = bytes.to_vec();
    appended.extend_from_slice(&deterministic_noise(31));
    cases.push(("append_noise".to_owned(), appended));
    for size in [0usize, 1, 3, 8, 31, 96] {
        cases.push((format!("noise_{size}"), deterministic_noise(size)));
    }
    cases
}

pub fn deterministic_noise(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| ((index * 37 + size * 11) & 0xff) as u8)
        .collect()
}

pub fn extra_mutations(seed: &[u8], count: usize) -> Vec<(String, Vec<u8>)> {
    (0..count)
        .map(|round| (format!("extra_{round}"), extra_mutation(seed, round)))
        .collect()
}

fn extra_mutation(seed: &[u8], round: usize) -> Vec<u8> {
    let mut bytes = if seed.is_empty() {
        deterministic_noise((round % 257) + 1)
    } else {
        seed.to_vec()
    };
    if !bytes.is_empty() {
        let position = (round.wrapping_mul(17).wrapping_add(bytes.len())) % bytes.len();
        bytes[position] ^= ((round.wrapping_mul(31) + 0xa5) & 0xff) as u8;
    }
    match round % 4 {
        0 => bytes,
        1 => {
            let new_len = round % (bytes.len() + 1);
            bytes.truncate(new_len);
            bytes
        }
        2 => {
            bytes.extend_from_slice(&deterministic_noise((round % 64) + 1));
            bytes
        }
        _ => deterministic_noise((round % 512) + 1),
    }
}

fn unique_positions(len: usize) -> Vec<usize> {
    let mut positions = BTreeSet::from([0, 1, len / 2, len.saturating_sub(1), len]);
    positions.retain(|position| *position <= len);
    positions.into_iter().collect()
}
