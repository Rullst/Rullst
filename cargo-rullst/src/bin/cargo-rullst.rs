#[cfg_attr(mutants, mutants::skip)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    cargo_rullst::run()
}
