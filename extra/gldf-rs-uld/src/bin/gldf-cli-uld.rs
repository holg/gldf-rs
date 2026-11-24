//! `gldf-cli` with DIALux `.uld` input.

fn main() -> anyhow::Result<()> {
    gldf_rs_uld::register();
    gldf_rs_cli::run()
}
