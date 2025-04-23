use rustc_attr_data_structures::AttributeKind;
use rustc_span::sym;

use super::{AcceptContext, SingleAttributeParser};
use crate::parser::ArgParser;

pub(crate) struct ProviderParser;

// FIXME(jdonszelmann): make these proper diagnostics
impl SingleAttributeParser for ProviderParser {
    const PATH: &'static [rustc_span::Symbol] = &[sym::provider];

    fn on_duplicate(_cx: &crate::context::AcceptContext<'_>, _first_span: rustc_span::Span) {}

    fn convert(_cx: &AcceptContext<'_>, args: &ArgParser<'_>) -> Option<AttributeKind> {
        assert!(args.no_args());
        Some(AttributeKind::Provider)
    }
}
