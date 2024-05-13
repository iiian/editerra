use thiserror::Error;

#[derive(Debug, Error)]
pub enum MapEdiError {
    #[error("IfThenSuppress target must be like NM104 or CLM05-1")]
    IfThenSuppress,
    #[error("exec/eval error: `{0}`")]
    ExprEngineErr(String),
    #[error(".`{0}`")]
    UnmetRequirement(String),
    #[error("could not load context: `{0}`")]
    LoadContextErr(String),
}

#[macro_export]
macro_rules! exp_errf {
    () => {
        |e| MapEdiError::ExprEngineErr(e.to_string())
    };
}
