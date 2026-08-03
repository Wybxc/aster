use anyhow::{Result, bail};
use typst::foundations::{Bytes, Content, Value};
use typst::introspection::MetadataElem;

fn declaration(content: &Content) -> Result<Option<Value>> {
    let mut declarations = Vec::new();
    let _ = content.traverse(&mut |element| {
        if element
            .label()
            .is_some_and(|label| *label.resolve() == *"endpoint")
            && let Some(metadata) = element.to_packed::<MetadataElem>()
        {
            declarations.push(metadata.value.clone());
        }
        std::ops::ControlFlow::<()>::Continue(())
    });

    if declarations.len() > 1 {
        bail!("endpoint template must contain exactly one <endpoint> declaration");
    }

    Ok(declarations.pop())
}

/// Whether a probe identified this source as an endpoint template.
pub(crate) fn is_declared(content: &Content) -> Result<bool> {
    Ok(declaration(content)?.is_some())
}

/// Extract the generated-file payload produced for one endpoint route.
pub(crate) fn extract(content: &Content) -> Result<Option<Bytes>> {
    match declaration(content)? {
        None => Ok(None),
        Some(Value::Str(content)) => Ok(Some(Bytes::from_string(content))),
        Some(Value::Bytes(content)) => Ok(Some(content)),
        Some(_) => bail!("endpoint metadata must contain a string or bytes"),
    }
}
