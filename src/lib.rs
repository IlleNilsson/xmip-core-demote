#![forbid(unsafe_code)]

use xmip_context::{ContextValue, MessageContext};
use xmip_contract::{ContractError, StructuredValue, StructureWriter};
use xmip_path::{Path, PathEngine};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demotion {
    pub context_key: String,
    pub target_path: Path,
}

pub trait ArtifactTarget {
    fn write_value(&mut self, path: &Path, value: &ContextValue) -> Result<(), String>;
}

pub fn apply_to_structure(
    context: &MessageContext,
    writer: &mut dyn StructureWriter,
    engine: &dyn PathEngine,
    demotions: &[Demotion],
) -> Result<(), ContractError> {
    for demotion in demotions {
        if let Some(value) = context.get(&demotion.context_key) {
            engine.write(writer, &demotion.target_path, convert(value.clone()))?;
        }
    }
    Ok(())
}

pub fn apply_to_artifact(
    context: &MessageContext,
    target: &mut dyn ArtifactTarget,
    demotions: &[Demotion],
) -> Result<(), String> {
    for demotion in demotions {
        if let Some(value) = context.get(&demotion.context_key) {
            target.write_value(&demotion.target_path, value)?;
        }
    }
    Ok(())
}

fn convert(value: ContextValue) -> StructuredValue {
    match value {
        ContextValue::Null => StructuredValue::Null,
        ContextValue::Bool(value) => StructuredValue::Bool(value),
        ContextValue::Integer(value) => StructuredValue::Integer(value),
        ContextValue::Decimal(value) => StructuredValue::Decimal(value),
        ContextValue::Text(value) => StructuredValue::Text(value),
        ContextValue::Binary(value) => StructuredValue::Binary(value),
    }
}
