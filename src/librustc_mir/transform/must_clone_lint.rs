// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(warnings)] // TODO

use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::indexed_vec::IndexVec;

use rustc::ty::maps::Providers;
use rustc::ty::{self, TyCtxt};
use rustc::hir;
use rustc::hir::def_id::DefId;
use rustc::lint::builtin::MUST_CLONE;
use rustc::mir::*;
use rustc::mir::visit::{PlaceContext, Visitor};

use syntax::ast;
use syntax::symbol::Symbol;

use std::rc::Rc;
use util;

pub struct MustCloneChecker<'a, 'tcx: 'a> {
    mir: &'a Mir<'tcx>,
    visibility_scope_info: &'a IndexVec<VisibilityScope, VisibilityScopeInfo>,
    violations: Vec<MustCloneViolation>,
    source_info: SourceInfo,
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    inherited_blocks: Vec<(ast::NodeId, bool)>,
}

impl<'a, 'gcx, 'tcx> MustCloneChecker<'a, 'tcx> {
    fn new(mir: &'a Mir<'tcx>,
           visibility_scope_info: &'a IndexVec<VisibilityScope, VisibilityScopeInfo>,
           tcx: TyCtxt<'a, 'tcx, 'tcx>,
           param_env: ty::ParamEnv<'tcx>) -> Self {
        Self {
            mir,
            visibility_scope_info,
            violations: vec![],
            source_info: SourceInfo {
                span: mir.span,
                scope: ARGUMENT_VISIBILITY_SCOPE
            },
            tcx,
            param_env,
            inherited_blocks: vec![],
        }
    }
}

impl<'a, 'tcx> Visitor<'tcx> for MustCloneChecker<'a, 'tcx> {
    fn visit_operand(&mut self,
                     operand: &Operand<'tcx>,
                     location: Location) {
        if let Operand::Copy(ref place) = *operand {
            panic!("oh shit itsa copy");
        }
    }
}

pub(crate) fn provide(providers: &mut Providers) {
    *providers = Providers {
        must_clone_result,
        ..*providers
    };
}

fn must_clone_result<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId)
                                   -> MustCloneCheckResult
{
    debug!("must_clone_violations({:?})", def_id);

    // NB: this borrow is valid because all the consumers of
    // `mir_built` force this.
    let mir = &tcx.mir_built(def_id).borrow();

    let visibility_scope_info = match mir.visibility_scope_info {
        ClearCrossCrate::Set(ref data) => data,
        ClearCrossCrate::Clear => {
            debug!("must_clone_violations: {:?} - remote, skipping", def_id);
            return MustCloneCheckResult {
                violations: Rc::new([]),
            }
        }
    };

    let param_env = tcx.param_env(def_id);
    let mut checker = MustCloneChecker::new(mir, visibility_scope_info, tcx, param_env);
    checker.visit_mir(mir);

    MustCloneCheckResult {
        violations: checker.violations.into(),
    }
}

pub fn must_clone_lint<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) {
    debug!("check_must_clone({:?})", def_id);

    // closures are handled by their parent fn.
    if tcx.is_closure(def_id) {
        return;
    }

    let MustCloneCheckResult {
        violations,
    } = tcx.must_clone_result(def_id);

    for &MustCloneViolation {
        source_info, lint_node_id, description
    } in violations.iter() {
        tcx.lint_node(MUST_CLONE,
                      lint_node_id,
                      source_info.span,
                      &format!("{} is implicitly copied here", &description[..]));
    }
}
