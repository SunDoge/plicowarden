use snafu::Snafu;
use zeroize::Zeroizing;
#[derive(Debug, Snafu)]
#[snafu(display("The operating system random source is unavailable."))]
pub struct RandomError;
fn random_index(max: usize) -> Result<usize, RandomError> {
    assert!(max > 0);
    let max = max as u64;
    let threshold = max.wrapping_neg() % max;
    loop {
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).map_err(|_| RandomError)?;
        let value = u64::from_le_bytes(bytes);
        if value >= threshold {
            return Ok((value % max) as usize);
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Password,
    Passphrase,
    Username,
}

const WORDS: &str = include_str!("../assets/eff-short-words.txt");

pub fn passphrase(count: usize) -> Result<Zeroizing<String>, RandomError> {
    let words: Vec<_> = WORDS.lines().collect();
    let mut result = Zeroizing::new(String::new());
    for index in 0..count.clamp(6, 12) {
        if index > 0 {
            result.push('-');
        }
        result.push_str(words[random_index(words.len())?]);
    }
    Ok(result)
}

/// A readable random identifier, not a password or a uniqueness guarantee.
pub fn username() -> Result<Zeroizing<String>, RandomError> {
    let words: Vec<_> = WORDS.lines().collect();
    Ok(Zeroizing::new(format!(
        "{}-{}-{:04}",
        words[random_index(words.len())?],
        words[random_index(words.len())?],
        random_index(10000)?
    )))
}

pub fn generate(length: usize, symbols: bool) -> Result<Zeroizing<String>, RandomError> {
    let groups: &[&[u8]] = if symbols {
        &[
            b"abcdefghijkmnopqrstuvwxyz",
            b"ABCDEFGHJKLMNPQRSTUVWXYZ",
            b"23456789",
            b"!@#$%^&*-_=+?",
        ]
    } else {
        &[
            b"abcdefghijkmnopqrstuvwxyz",
            b"ABCDEFGHJKLMNPQRSTUVWXYZ",
            b"23456789",
        ]
    };
    let alphabet = groups.concat();
    let length = length.clamp(8, 128);
    let mut result = Zeroizing::new(Vec::<u8>::with_capacity(length));
    for group in groups {
        result.push(group[random_index(group.len())?]);
    }
    while result.len() < length {
        result.push(alphabet[random_index(alphabet.len())?]);
    }
    for i in (1..length).rev() {
        let j = random_index(i + 1)?;
        result.swap(i, j);
    }
    // The alphabet is ASCII; never put the generated value in an error/debug message.
    Ok(Zeroizing::new(
        String::from_utf8_lossy(&result).into_owned(),
    ))
}
