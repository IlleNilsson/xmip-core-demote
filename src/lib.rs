#![forbid(unsafe_code)]

use context::MessageContext;

use contract::ContractError;
use path::{CompiledPath, Path, PathEngine, Rewriting};
use xcore::ScalarValue;

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

impl Demotion {
    /// This demotion with its Path compiled through `engine`, once, when
    /// configuration is read — when it writes into the payload. `None` for
    /// every other target: a header or a transport property is not the
    /// payload's to write, and silently writing it there instead would
    /// corrupt content to satisfy a routing concern.
    ///
    /// # Errors
    /// The Path's language is not loaded, or refuses its expression.
    pub fn compile(&self, engine: &PathEngine) -> Result<Option<PayloadDemotion>, ContractError> {
        if self.target != DemotionTarget::PayloadElement {
            return Ok(None);
        }
        Ok(Some(PayloadDemotion {
            context_key: self.context_key.clone(),
            path: engine.compile(&self.target_path)?,
        }))
    }
}

/// A demotion into the payload, its Path compiled.
#[derive(Debug)]
pub struct PayloadDemotion {
    pub context_key: String,
    pub path: CompiledPath,
}

pub trait ArtifactTarget {
    fn write_value(
        &mut self,
        target: DemotionTarget,
        path: &Path,
        value: &ScalarValue,
    ) -> Result<(), String>;
}

/// Write each demoted value into the payload being rewritten; a context key
/// the Message does not hold writes nothing. The rewrite produces the new
/// Stream (ADR-0013).
///
/// # Errors
/// A Path could not write its value.
pub fn apply_to_structure(
    context: &MessageContext,
    rewriting: &mut Rewriting,
    demotions: &[PayloadDemotion],
) -> Result<(), ContractError> {
    for demotion in demotions {
        if let Some(value) = context.get(&demotion.context_key) {
            // A promoted property and a structured field are one type
            // (core::ScalarValue), so it writes straight in with no conversion.
            demotion.path.write(rewriting, value.clone())?;
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
            _value: &ScalarValue,
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
            .with_value("order.id", ScalarValue::Text("A-1".into()))
            .with_value("trace.id", ScalarValue::Text("T-1".into()));

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

    /// A language whose expression is `key=` in text, the value after it
    /// rewritten up to the next `;`.
    struct KeyValue;

    struct Key(String);

    impl path::PathLanguage for KeyValue {
        fn language(&self) -> &'static str {
            "key-value"
        }

        fn compile(
            &self,
            expression: &str,
        ) -> Result<Box<dyn path::CompiledExpression>, ContractError> {
            Ok(Box::new(Key(format!("{expression}="))))
        }
    }

    impl path::CompiledExpression for Key {
        fn read(&self, _: &path::Content<'_>) -> Result<Option<ScalarValue>, ContractError> {
            Ok(None)
        }

        fn write(
            &self,
            rewriting: &mut Rewriting,
            value: ScalarValue,
        ) -> Result<(), ContractError> {
            let written = value.text().ok_or_else(|| ContractError::new("no text"))?;
            let text = rewriting.form_mut::<String>()?;
            let start = text
                .find(&self.0)
                .ok_or_else(|| ContractError::new("absent"))?
                + self.0.len();
            let end = text[start..].find(';').map_or(text.len(), |at| start + at);
            text.replace_range(start..end, &written);
            Ok(())
        }
    }

    #[test]
    fn only_a_payload_demotion_compiles_and_it_rewrites_the_stream() {
        let engine = PathEngine::new(vec![Box::new(KeyValue)]);
        let into_payload = |key: &str| Demotion {
            context_key: key.to_string(),
            target: DemotionTarget::PayloadElement,
            target_path: Path::new("key-value", key),
        };
        assert!(
            demotion("order", DemotionTarget::StreamHeader)
                .compile(&engine)
                .expect("not the payload's")
                .is_none()
        );
        let demotions: Vec<PayloadDemotion> = ["status", "absent"]
            .map(|key| into_payload(key).compile(&engine).expect("compiles"))
            .into_iter()
            .flatten()
            .collect();
        let context =
            MessageContext::new().with_value("status", ScalarValue::Text("closed".into()));
        let source = stream::Stream::new(
            xcore::StreamId::new(1),
            b"order=A-1;status=open".to_vec(),
            None,
        );
        let mut rewriting = Rewriting::of(&source, xcore::StreamId::new(2));

        apply_to_structure(&context, &mut rewriting, &demotions).expect("writes");

        let written = rewriting.finish().expect("finishes");
        assert_eq!(written.bytes(), b"order=A-1;status=closed");
        assert!(
            Demotion {
                target_path: Path::new("xpath", "/order"),
                ..into_payload("status")
            }
            .compile(&engine)
            .is_err()
        );
    }
}
