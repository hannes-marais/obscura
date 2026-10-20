//! Synchronous browser dialog hook. The embedder must keep its protocol reader
//! independent of the isolate thread while this callback waits for a choice.
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogRequest {
    pub kind: String,
    pub message: String,
    pub default_prompt: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogChoice {
    pub accept: bool,
    pub prompt_text: String,
}

pub type DialogHandler = Arc<dyn Fn(DialogRequest) -> Result<DialogChoice, String> + Send + Sync>;
