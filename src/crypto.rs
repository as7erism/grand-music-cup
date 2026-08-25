use std::fmt::Display;

use base64::prelude::*;
use rand::{
    SeedableRng, TryCryptoRng, TryRng, rand_core,
    rngs::{StdRng, SysRng},
};

pub fn init_rng() -> StdRng {
    StdRng::try_from_rng(&mut SysRng).expect("stdrng operations are infallible")
}

pub fn random_bytes<const LEN: usize>(rng: &mut StdRng) -> [u8; LEN] {
    let mut buffer: [u8; _] = [0; LEN];
    rng.try_fill_bytes(&mut buffer)
        .expect("stdrng operations are infallible");
    buffer
}
