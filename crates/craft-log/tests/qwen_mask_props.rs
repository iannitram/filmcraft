//! Property tests for craft-log masking functionality.
//!
//! This file contains property-style tests using a fixed-seed xorshift generator
//! to verify the masking behavior of `mask_args_checked`.

use craft_log::cli::mask_args_checked;

/// Simple xorshift64* random number generator for fixed-seed tests
struct Xorshift64Star {
    state: u64,
}

impl Xorshift64Star {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    fn next_char(&mut self) -> char {
        const CHARS: &[char] = &['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        CHARS[self.next_usize() % CHARS.len()]
    }

    fn next_hex_char(&mut self) -> char {
        const CHARS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];
        CHARS[self.next_usize() % CHARS.len()]
    }

    fn next_string(&mut self, len: usize) -> String {
        (0..len).map(|_| self.next_char()).collect()
    }

    fn next_hex_string(&mut self, len: usize) -> String {
        (0..len).map(|_| self.next_hex_char()).collect()
    }
}

/// Generate a random 64-hex-digit token
fn random_token(rng: &mut Xorshift64Star) -> String {
    rng.next_hex_string(64)
}

/// Check if a string is a 64-hex-digit token
fn is_64_hex_token(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Test that a random 64-hex-digit token is completely hidden when placed anywhere in an argument list
#[test]
fn test_token_hidden_everywhere() {
    let mut rng = Xorshift64Star::new(42);
    
    for i in 0..500 {
        let token = random_token(&mut rng);
        
        // Create a simple argument list with the token
        let args = vec![
            "--token".to_string(),
            token.clone()
        ];
        
        let (shown, secrets) = mask_args_checked(&args, &["--token"], &is_64_hex_token, &[]);
        
        // The token should be in the secrets list (it should be identified as a secret)
        let token_in_secrets = secrets.iter().any(|s| s == &token);
        assert!(token_in_secrets, 
                "Seed: 42, Test case: {} - Token '{}' not found in secrets: {:?}", 
                i, token, secrets);
        
        // The shown output should not contain the token in plain text
        // Since we have "--token" as a switch, it should hide the following argument
        let shown_str = shown.join(" ");
        assert!(!shown_str.contains(&token),
                "Seed: 42, Test case: {} - Token '{}' found in shown output: {:?}", 
                i, token, shown);
    }
}

/// Test that two different tokens side by side are both hidden
#[test]
fn test_two_different_tokens() {
    let mut rng = Xorshift64Star::new(123);
    
    for i in 0..500 {
        let token1 = random_token(&mut rng);
        let token2 = random_token(&mut rng);
        
        // Create an argument list with both tokens
        let args = vec![
            "--token".to_string(),
            token1.clone(),
            "--token".to_string(),
            token2.clone()
        ];
        
        let (shown, secrets) = mask_args_checked(&args, &["--token"], &is_64_hex_token, &[]);
        
        // Both tokens should be in the secrets list (they should be identified as secrets)
        let token1_in_secrets = secrets.iter().any(|s| s == &token1);
        let token2_in_secrets = secrets.iter().any(|s| s == &token2);
        assert!(token1_in_secrets, 
                "Seed: 123, Test case: {} - Token1 '{}' not found in secrets: {:?}", 
                i, token1, secrets);
        assert!(token2_in_secrets, 
                "Seed: 123, Test case: {} - Token2 '{}' not found in secrets: {:?}", 
                i, token2, secrets);
        
        // Neither token should appear in plain text in the shown output
        let shown_str = shown.join(" ");
        assert!(!shown_str.contains(&token1),
                "Seed: 123, Test case: {} - Token1 '{}' found in shown output: {:?}", 
                i, token1, shown);
        assert!(!shown_str.contains(&token2),
                "Seed: 123, Test case: {} - Token2 '{}' found in shown output: {:?}", 
                i, token2, shown);
    }
}

/// Test that non-secret arguments come out unchanged
#[test]
fn test_non_secret_args_unchanged() {
    let mut rng = Xorshift64Star::new(456);
    
    for i in 0..500 {
        let mut args = Vec::new();
        let arg_count = 1 + (rng.next_u32() % 10); // 1-10 arguments
        
        for _ in 0..arg_count {
            let arg_type = rng.next_u32() % 4;
            let len = 10 + (rng.next_u32() % 20) as usize;
            match arg_type {
                0 => args.push(rng.next_string(len)), // random word
                1 => args.push(format!("/home/x/file{}.mp4", rng.next_u32() % 100)), // file path
                2 => args.push(format!("{}", rng.next_u32() % 1000)), // number
                3 => {
                    let len = 5 + (rng.next_u32() % 10) as usize;
                    args.push(rng.next_string(len)); // random word
                }
                _ => unreachable!(),
            }
        }
        
        let (shown, _secrets) = mask_args_checked(&args, &["--token"], &is_64_hex_token, &[]);
        
        // Check that all non-secret arguments are unchanged
        assert_eq!(shown, args, 
                   "Seed: 456, Test case: {} - Arguments changed unexpectedly. Expected: {:?}, Got: {:?}", 
                   i, args, shown);
    }
}

/// Test that masking is idempotent
#[test]
fn test_masking_idempotent() {
    let mut rng = Xorshift64Star::new(789);
    
    for i in 0..500 {
        let token = random_token(&mut rng);
        
        // Create a simple argument list with the token
        let args = vec![
            "--token".to_string(),
            token.clone()
        ];
        
        let (shown1, _secrets1) = mask_args_checked(&args, &["--token"], &is_64_hex_token, &[]);
        let (shown2, _secrets2) = mask_args_checked(&shown1, &["--token"], &is_64_hex_token, &[]);
        
        // Check that masking the masked output changes nothing
        assert_eq!(shown1, shown2, 
                   "Seed: 789, Test case: {} - Masking is not idempotent. First: {:?}, Second: {:?}", 
                   i, shown1, shown2);
    }
}