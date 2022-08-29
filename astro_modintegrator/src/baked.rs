macro_rules! baked_path {
    () => {
        concat!(env!("OUT_DIR"), "/baked/")
    };
}

pub(crate) const CORE_MOD: &[u8] =
    include_bytes!(concat!(baked_path!(), "800-CoreMod-0.1.0_P.pak"));
