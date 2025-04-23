use rustc_attr_data_structures::{AttributeKind, Provider};
use rustc_span::{sym, Symbol};

use super::{AcceptContext, SingleAttributeParser};
use crate::parser::ArgParser;
use crate::session_diagnostics;
pub(crate) struct ProviderParser;

// FIXME(jdonszelmann): make these proper diagnostics
impl SingleAttributeParser for ProviderParser {
    const PATH: &'static [Symbol] = &[sym::provider];

    fn on_duplicate(cx: &crate::context::AcceptContext<'_>, first_span: rustc_span::Span) {
        cx.emit_err(session_diagnostics::UnusedMultiple {
            this: cx.attr_span,
            other: first_span,
            name: sym::provider,
        });
    }

    fn convert(cx: &AcceptContext<'_>, args: &ArgParser<'_>) -> Option<AttributeKind> {
        let opt_id = (|| -> Option<Symbol> {
          // Parse out `(id = "...")`
          args.list()?.single()?.meta_item()?.word_is(sym::id)?.name_value()?.value_as_str()
        })();
        let Some(provider_id) = opt_id else {
          cx.dcx().span_err(cx.attr_span, "expected `#[provider(id = \"...\")");
          return None;
        };
        Some(AttributeKind::Provider(Provider { provider_id, span: cx.attr_span }))
    }
}
