#![forbid(unsafe_code)]

use context::{ContextValue, MessageContext};
use contract::{ContractError, StructureWriter};
use path::{Path, PathEngine};

/// Which surface a value is written onto.
///
/// Demotion is not one operation. Writing a correlation key into an element of
/// the payload changes the Stream and so produces a new one; writing it into a
/// transport property changes nothing but the envelope around it. The target
/// says which of those is happening, and only `PayloadElement` costs a Stream.
///
/// Arrived from the platform repository's `src/contracts.rs` on 2026-08-26.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemotionTarget {
    /// A header of the Stream itself.
    StreamHeader,
    /// Metadata carried beside the Stream, not inside it.
    StreamMetadata,
    /// Inside the payload. This one rewrites content, so under ADR-0013 it
    /// produces a new Stream rather than editing the one in hand.
    PayloadElement,
    /// The envelope the Stream travels in.
    Envelope,
    /// A property of the transport, discarded when the transport is done.
    TransportProperty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demotion {
    pub context_key: String,
    pub target: DemotionTarget,
    pub target_path: Path,
}

pub trait ArtifactTarget {
    fn write_value(
        &mut self,
        target: DemotionTarget,
        path: &Path,
        value: &ContextValue,
    ) -> Result<(), String>;
}

pub fn apply_to_structure(
    context: &MessageContext,
    writer: &mut dyn StructureWriter,
    engine: &dyn PathEngine,
    demotions: &[Demotion],
) -> Result<(), ContractError> {
    for demotion in demotions {
        // A StructureWriter reaches inside the payload and nowhere else. A
        // header or a transport property is not this function's to write, and
        // silently writing it into the payload instead would corrupt content
        // to satisfy a routing concern.
        if demotion.target != DemotionTarget::PayloadElement {
            continue;
        }

        if let Some(value) = context.get(&demotion.context_key) {
            // A promoted property and a structured field are one type now
            // (core::ScalarValue), so it writes straight in with no conversion.
            engine.write(writer, &demotion.target_path, value.clone())?;
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
            target.write_value(demotion.target, &demotion.target_path, value)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        written: Vec<(DemotionTarget, String)>,
    }

    impl ArtifactTarget for Recorder {
        fn write_value(
            &mut self,
            target: DemotionTarget,
            path: &Path,
            _value: &ContextValue,
        ) -> Result<(), String> {
            self.written.push((target, path.expression.clone()));
            Ok(())
        }
    }

    fn demotion(key: &str, target: DemotionTarget) -> Demotion {
        Demotion {
            context_key: key.to_string(),
            target,
            target_path: Path::new("direct", format!("/{key}")),
        }
    }

    #[test]
    fn an_artifact_target_is_told_which_surface_to_write() {
        let context = MessageContext::new()
            .with_value("order.id", ContextValue::Text("A-1".into()))
            .with_value("trace.id", ContextValue::Text("T-1".into()));

        let mut recorder = Recorder::default();
        let demotions = [
            demotion("order.id", DemotionTarget::StreamHeader),
            demotion("trace.id", DemotionTarget::TransportProperty),
        ];

        apply_to_artifact(&context, &mut recorder, &demotions).expect("write");

        assert_eq!(recorder.written.len(), 2);
        assert_eq!(recorder.written[0].0, DemotionTarget::StreamHeader);
        assert_eq!(recorder.written[1].0, DemotionTarget::TransportProperty);
    }

    #[test]
    fn a_missing_context_key_writes_nothing() {
        let mut recorder = Recorder::default();

        apply_to_artifact(
            &MessageContext::new(),
            &mut recorder,
            &[demotion("absent", DemotionTarget::Envelope)],
        )
        .expect("write");

        assert!(recorder.written.is_empty());
    }
}
