use std::{format, fs::File, io::{self, Write}, print, println, process::exit};

use ed25519_dalek::{SigningKey, pkcs8::{EncodePrivateKey, EncodePublicKey}};
use filepipe::keys::generate_random_string;

use getrandom::{SysRng, rand_core::{UnwrapErr}};
use ssh_key::LineEnding;

fn main() {
    let mut name = format!("fpk_{}", generate_random_string(8));

    println!("FilePipe - Woxell, key generation tool");
    println!("This tool generates ED25519 key pair required for FilePipe authentication");

    print!("Enter a file in which to save the key ({name}): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let input = input.trim();
    if !input.is_empty() {
        name = input.to_string();
    }
    
    let mut csprng = UnwrapErr(SysRng);

    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let private_key = signing_key.to_pkcs8_pem(LineEnding::LF)
        .unwrap_or_else(|_| {
            println!("failed to generate private key");
            exit(1);
        });
    let public_key = verifying_key.to_public_key_pem(LineEnding::LF)
        .unwrap_or_else(|_| {
            println!("failed to generate public key");
            exit(1);
        });

    let mut file = File::create(format!("{name}.pub"))
        .unwrap_or_else(|_| {
            println!("failed to open output file for public key");
            exit(1);
        });

    match file.write(public_key.as_bytes()) {
        Ok(_) => {},
        Err(_) => {
            println!("failed to write to output file for public key");
            exit(1);
        }
    };

    file = File::create(&name)
        .unwrap_or_else(|_| {
            println!("failed to open output file for private key");
            exit(1);
        });

    match file.write(private_key.as_bytes()) {
        Ok(_) => {},
        Err(_) => {
            println!("failed to write to output file for private key");
            exit(1);
        }
    };

    println!("Generated keys: {name}");
}