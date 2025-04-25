#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_ast::Crate;
use rustc_driver::{Compilation, catch_fatal_errors, run_compiler};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{Visitor, walk_item, walk_qpath};
use rustc_hir::{self as hir, HirId};
use rustc_interface::interface::{Compiler, Config};
use rustc_middle::ty::{self, TyCtxt};
use rustc_middle::util::Providers;
use rustc_session::Session;
use rustc_session::parse::ParseSess;
use rustc_span::fatal_error::FatalError;
use rustc_span::{Span, kw};

struct EcdysisCallbacks {
    // FIXME probably some state :)
}

fn override_queries(_session: &Session, providers: &mut Providers) {
    // FIXME override as-needed? maybe we won't need this if we can feed everything?
    providers.queries = rustc_middle::query::Providers {
        resolved_provided_item: |tcx, def_id| resolve_provided_item(tcx, def_id),
        ..providers.queries
    };
}

fn resolve_provided_item(tcx: TyCtxt<'_>, def_id: LocalDefId) -> Option<DefId> {
    let span = tcx.def_span(def_id);
    let dcx = tcx.dcx();

    let &[arg] = tcx.provided_item_args(def_id).as_slice() else {
        dcx.span_err(span, "expected exactly one argument");
        return None;
    };

    // For now, act like a type alias to the provided argument.
    let ty_def = tcx.at(span).create_def(def_id, Some(kw::Empty), DefKind::TyAlias);
    ty_def.type_of(ty::EarlyBinder::bind(arg));
    ty_def.feed_hir();

    Some(ty_def.def_id().to_def_id())
}

struct TyProviderCollector<'tcx> {
    tcx: TyCtxt<'tcx>,
    // parent_type_bodies: ,
}

impl<'tcx> Visitor<'tcx> for TyProviderCollector<'tcx> {
    type NestedFilter = rustc_middle::hir::nested_filter::All;
    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.tcx
    }
    // FIXME: record all uses of `ProvidedTy` and their input type generics. We need to *try* to
    // make a well-ordered list of what `ProvidedTy`s are dependent on what other `ProvidedTy`s so
    // that we can build them in the right order, as well as recording the non-`ProvidedTy`s they
    // reference so that we can produce their C++ bindings in the same order (breaking cycles w/
    // forward declarations where possible).
    //
    // There's some similar logic today in cc_bindings_from_rs.

    fn visit_qpath(
        &mut self,
        qpath: &'tcx hir::QPath<'tcx>,
        id: HirId,
        _span: Span,
    ) -> Self::Result {
        walk_qpath(self, qpath, id);
        let hir::QPath::Resolved(_, path) = qpath else {
            return;
        };
        let Res::Def(DefKind::ProvidedTy, _def_id) = path.res else {
            return;
        };
        // dbg!(_path, _def_id);
        // FIXME: record the ProvidedTy and its input type generics.
    }

    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
        walk_item(self, item);
        match item.kind {
            rustc_hir::ItemKind::TyProvider { .. } => {
                // dbg!("TyProvider", item);
            }
            _ => {}
        }
    }
}

impl rustc_driver::Callbacks for EcdysisCallbacks {
    fn config(&mut self, config: &mut Config) {
        config.psess_created = Some(Box::new(|_psess: &mut ParseSess| {
            // FIXME: mutate psess as-needed.
            // psess.file_depinfo should be populated with the file paths of the C++ files.
            //
            // We could consider mutating the source map to contain the C++ files as well if we ever
            // want to point to them from rustc spans.
        }));
        // FIXME: hash_untracked_state probably to make incremental work someday.
        // FIXME: register_lints
        // Q: why is `override_queries` a `fn` rather than `Box::FnOnce`? Why can't it have state?
        config.override_queries = Some(override_queries);
    }

    fn after_crate_root_parsing(
        &mut self,
        _compiler: &Compiler,
        _krate: &mut Crate,
    ) -> Compilation {
        // FIXME: collect any syntactic (pre-macro-expansion / name resolution!) misuse.
        // probably nothing to do here for now.
        Compilation::Continue
    }

    fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        // FIXME(ecdysis): For each use of a `TyProvider` in a path, record its generic parameters.
        let mut visitor = TyProviderCollector { tcx };
        tcx.hir_walk_toplevel_module(&mut visitor);

        // pluto incremental build for dynamic deps? https://www.pl.informatik.uni-mainz.de/files/2019/04/pluto-incremental-build.pdf
        // tcx.at().create_def();
        Compilation::Continue
    }

    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, _tcx: TyCtxt<'tcx>) -> Compilation {
        Compilation::Continue
    }
}

fn main() {
    let rustc_args: Vec<String> = std::env::args().collect();
    let mut driver_callbacks = EcdysisCallbacks {};

    // The Rust compiler unwinds with a special sentinel value to abort compilation on
    // fatal errors. We use `catch_fatal_errors` to 1) catch such panics and
    // translate them into a Result, and 2) resume and propagate other panics.
    let catch_fatal_errors_result: Result<(), FatalError> =
        catch_fatal_errors(|| run_compiler(&*rustc_args, &mut driver_callbacks));

    match catch_fatal_errors_result {
        Ok(()) => {}
        // We can ignore the `Err` payloads because the error types have only one value.
        _ => panic!("Errors reported by Rust compiler."),
    };
}
