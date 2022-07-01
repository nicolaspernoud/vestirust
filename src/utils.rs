use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn random_string(size: usize) -> std::string::String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}
