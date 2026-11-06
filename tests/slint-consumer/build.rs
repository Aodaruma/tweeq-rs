use std::{collections::HashMap, env, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let entry = tweeq_slint_build::export_ui(&output.join("tweeq-ui"))?;
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(HashMap::from([("tweeq".into(), entry)]));
    slint_build::compile_with_config("app.slint", config)?;
    Ok(())
}
