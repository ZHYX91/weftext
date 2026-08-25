//! First-party bridge for the `DeepSeek` Harness SDK JSON-RPC runtime.

mod client;
mod protocol;

pub use client::{DshClient, DshError};
pub use protocol::{
    DSH_RUNTIME_NAME, DSH_WIRE_VERSION, DshCompatibilityPolicy, DshInitialize, DshPrompt,
    DshPromptReceipt,
};
