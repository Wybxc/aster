use anyhow::{Result, bail};
use typst::foundations::{Bytes, Label, Selector, Value};
use typst::introspection::{Introspector, MetadataElem};

fn declaration(introspector: &dyn Introspector) -> Result<Option<Value>> {
    let selector =
        Selector::Label(Label::construct("endpoint".into()).expect("endpoint label is non-empty"));
    let mut declarations = introspector
        .query(&selector)
        .into_iter()
        .filter_map(|element| {
            element
                .to_packed::<MetadataElem>()
                .map(|metadata| metadata.value.clone())
        });
    let declaration = declarations.next();

    if declarations.next().is_some() {
        bail!("endpoint template must contain exactly one <endpoint> declaration");
    }

    Ok(declaration)
}

/// Whether a probe identified this source as an endpoint template.
pub(crate) fn is_declared(introspector: &dyn Introspector) -> Result<bool> {
    Ok(declaration(introspector)?.is_some())
}

/// Extract the generated-file payload produced for one endpoint route.
pub(crate) fn extract(introspector: &dyn Introspector) -> Result<Option<Bytes>> {
    match declaration(introspector)? {
        None => Ok(None),
        Some(Value::Str(content)) => Ok(Some(Bytes::from_string(content))),
        Some(Value::Bytes(content)) => Ok(Some(content)),
        Some(_) => bail!("endpoint metadata must contain a string or bytes"),
    }
}
