use base64::prelude::*;
use rand::{SeedableRng, TryRng, rngs::{StdRng, SysRng}};

const SECRET_LEN: usize = 32;

pub fn generate_secret() -> String {
    let mut buffer: [u8; _] = [0; SECRET_LEN];
    let mut rng = StdRng::try_from_rng(&mut SysRng).expect("seeding rng failed");
    rng.try_fill_bytes(&mut buffer).expect("filling buffer with random bytes failed");
    BASE64_STANDARD.encode(buffer)
}
