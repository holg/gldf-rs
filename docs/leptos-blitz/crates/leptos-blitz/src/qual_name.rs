//! Same helper dioxus-native-dom uses (see their lib.rs).

use blitz_dom::QualName;
use markup5ever::{local_name, namespace_url, ns, LocalName, Namespace};

pub fn qual_name(local: &str, namespace: Option<&str>) -> QualName {
    QualName {
        prefix: None,
        ns: namespace.map(Namespace::from).unwrap_or(ns!(html)),
        local: LocalName::from(local),
    }
}
