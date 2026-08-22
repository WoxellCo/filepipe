use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngExt;
use ssh_key::PublicKey;
use std::iter;

pub fn generate_random_string(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    let one_char = || CHARSET[rng.random_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(len).collect()
}

pub fn verify_signature(
    public_key_bytes: &[u8; 32],
    message: &[u8],
    signature_bytes: &[u8; 64],
) -> Result<(), ed25519_dalek::SignatureError> {
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)?;
    let signature = Signature::from_bytes(signature_bytes);

    verifying_key.verify(message, &signature)
}

pub fn parse_ssh_ed25519_pub_key(line: &str) -> Result<[u8; 32], ssh_key::Error> {
    let public_key = PublicKey::from_openssh(line)?;
    let ed25519_key = public_key
        .key_data()
        .ed25519()
        .ok_or(ssh_key::Error::AlgorithmUnknown)?;
    Ok(ed25519_key.0)
}
