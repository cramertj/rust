// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use fmt_macros::{Parser, Piece, Position};

use hir::def_id::DefId;
use traits;
use ty::{self, ToPredicate, TyCtxt};
use util::common::ErrorReported;
use util::nodemap::FxHashMap;

use syntax::ast::{LitKind, MetaItem, Name, NestedMetaItem};
use syntax::attr;
use syntax_pos::Span;
use syntax_pos::symbol::InternedString;

#[derive(Clone, Debug)]
pub struct OnUnimplementedFormatString(InternedString);

#[derive(Debug)]
pub struct OnUnimplementedDirective {
    pub condition: Option<MetaItem>,
    pub subcommands: Vec<OnUnimplementedDirective>,
    pub message: Option<OnUnimplementedFormatString>,
    pub label: Option<OnUnimplementedFormatString>,
}

pub struct OnUnimplementedNote {
    pub message: Option<String>,
    pub label: Option<String>,
}

impl OnUnimplementedNote {
    pub fn empty() -> Self {
        OnUnimplementedNote { message: None, label: None }
    }
}

fn parse_error(tcx: TyCtxt, span: Span,
               message: &str,
               label: &str,
               note: Option<&str>)
               -> ErrorReported
{
    let mut diag = struct_span_err!(
        tcx.sess, span, E0232, "{}", message);
    diag.span_label(span, label);
    if let Some(note) = note {
        diag.note(note);
    }
    diag.emit();
    ErrorReported
}

// Resolves `matches("TraitName", Self = "TypeName")` clauses to a trait name and a self name
fn names_from_matches_clause(nested_items: &[NestedMetaItem]) -> Result<(Name, Name), &'static str> {
    let mut bound_name = None;
    let mut self_name = None;

    for nested_item in nested_items {
        if let Some(lit) = nested_item.literal() {
            if let LitKind::Str(name, _) = lit.node {
                // This is a trait path like
                // matches("IsNoneError", Self="T")
                //          ^^^^^^^^^^^
                if bound_name.is_some() {
                    Err("Multiple string literals provided to rustc_on_unimplemented matches clause")?;
                }
                bound_name = Some(name);
            }
        } else if let Some((key, lit)) = nested_item.name_value_literal() {
            if key == "Self" {
                if let LitKind::Str(name, _) = lit.node {
                    // This is a self type specification like
                    // matches("IsNoneError", Self="T")
                    //                        ^^^^^^^^
                    if self_name.is_some() {
                        Err("Multiple Self types provided to rustc_on_unimplemented matches clause")?;
                    }
                    self_name = Some(name);
                }
            }
        }
    }

    Ok((
        bound_name.ok_or("matches clause should have a bound literal")?,
        self_name.ok_or("matches clause should have a self KV-pair")?,
    ))
}

impl<'a, 'gcx, 'tcx> OnUnimplementedDirective {
    pub fn parse(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                 trait_def_id: DefId,
                 items: &[NestedMetaItem],
                 span: Span,
                 is_root: bool)
                 -> Result<Self, ErrorReported>
    {
        let mut errored = false;
        let mut item_iter = items.iter();

        let condition = if is_root {
            None
        } else {
            let cond = item_iter.next().ok_or_else(|| {
                parse_error(tcx, span,
                            "empty `on`-clause in `#[rustc_on_unimplemented]`",
                            "empty on-clause here",
                            None)
            })?.meta_item().ok_or_else(|| {
                parse_error(tcx, span,
                            "invalid `on`-clause in `#[rustc_on_unimplemented]`",
                            "invalid on-clause here",
                            None)
            })?;
            attr::eval_condition_with_custom_list_handler(
                cond, &tcx.sess.parse_sess, &mut |_| true,
                &mut |attribute, nested_list| {
                    if attribute.name != "matches" {
                        return None;
                    }

                    // TODO: bother with proper error for internal feature?
                    names_from_matches_clause(nested_list).unwrap();

                    Some(true)
                });
            Some(cond.clone())
        };

        let mut message = None;
        let mut label = None;
        let mut subcommands = vec![];
        for item in item_iter {
            if item.check_name("message") && message.is_none() {
                if let Some(message_) = item.value_str() {
                    message = Some(OnUnimplementedFormatString::try_parse(
                        tcx, trait_def_id, message_.as_str(), span)?);
                    continue;
                }
            } else if item.check_name("label") && label.is_none() {
                if let Some(label_) = item.value_str() {
                    label = Some(OnUnimplementedFormatString::try_parse(
                        tcx, trait_def_id, label_.as_str(), span)?);
                    continue;
                }
            } else if item.check_name("on") && is_root &&
                message.is_none() && label.is_none()
            {
                if let Some(items) = item.meta_item_list() {
                    if let Ok(subcommand) =
                        Self::parse(tcx, trait_def_id, &items, item.span, false)
                    {
                        subcommands.push(subcommand);
                    } else {
                        errored = true;
                    }
                    continue
                }
            }

            // nothing found
            parse_error(tcx, item.span,
                        "this attribute must have a valid value",
                        "expected value here",
                        Some(r#"eg `#[rustc_on_unimplemented = "foo"]`"#));
        }

        if errored {
            Err(ErrorReported)
        } else {
            Ok(OnUnimplementedDirective { condition, message, label, subcommands })
        }
    }


    pub fn of_item(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                   trait_def_id: DefId,
                   impl_def_id: DefId)
                   -> Result<Option<Self>, ErrorReported>
    {
        let attrs = tcx.get_attrs(impl_def_id);

        let attr = if let Some(item) =
            attrs.into_iter().find(|a| a.check_name("rustc_on_unimplemented"))
        {
            item
        } else {
            return Ok(None);
        };

        let result = if let Some(items) = attr.meta_item_list() {
            Self::parse(tcx, trait_def_id, &items, attr.span, true).map(Some)
        } else if let Some(value) = attr.value_str() {
            Ok(Some(OnUnimplementedDirective {
                condition: None,
                message: None,
                subcommands: vec![],
                label: Some(OnUnimplementedFormatString::try_parse(
                    tcx, trait_def_id, value.as_str(), attr.span)?)
            }))
        } else {
            return Err(parse_error(tcx, attr.span,
                                   "`#[rustc_on_unimplemented]` requires a value",
                                   "value required here",
                                   Some(r#"eg `#[rustc_on_unimplemented = "foo"]`"#)));
        };
        debug!("of_item({:?}/{:?}) = {:?}", trait_def_id, impl_def_id, result);
        result
    }

    pub fn evaluate(&self,
                    tcx: TyCtxt<'a, 'gcx, 'tcx>,
                    trait_ref: ty::TraitRef<'tcx>,
                    options: &[(&str, Option<&str>)])
                    -> OnUnimplementedNote
    {
        let mut message = None;
        let mut label = None;
        info!("evaluate({:?}, trait_ref={:?}, options={:?})",
              self, trait_ref, options);

        for command in self.subcommands.iter().chain(Some(self)).rev() {
            if let Some(ref condition) = command.condition {
                if !attr::eval_condition_with_custom_list_handler(
                    condition, &tcx.sess.parse_sess,
                    &mut |c| {
                        options.contains(&(&c.name().as_str(),
                                           match c.value_str().map(|s| s.as_str()) {
                                               Some(ref s) => Some(s),
                                               None => None
                                           }))
                    },
                    &mut |attribute, nested_list| {
                        if attribute.name != "matches" {
                            return None;
                        }

                        let (trait_bound_name, self_name) = names_from_matches_clause(nested_list).unwrap();
                        let matches_resolutions = tcx.matches_resolutions(trait_ref.def_id)
                            .expect(&format!("No matches resolutions found for trait {:?}", trait_ref.def_id));

                        let trait_bound_id =
                            matches_resolutions.iter()
                                .find(|res| res.0 == trait_bound_name)
                                .map(|res| res.1)
                                .expect("no resolution for matches bound");

                        let self_id =
                            matches_resolutions.iter()
                                .find(|res| res.0 == self_name)
                                .map(|res| res.1)
                                .expect("no resolution for matches self type");

                        let self_ty = tcx.type_of(self_id);

                        let did_match = tcx.infer_ctxt().enter(|inferctxt| {
                            let mut selcx = traits::SelectionContext::new(&inferctxt);
                            let cause = traits::ObligationCause::new(
                                            attribute.span(),
                                            tcx.hir.as_local_node_id(trait_ref.def_id).unwrap(),
                                            traits::ObligationCauseCode::MiscObligation);

                            let predicate = ty::TraitRef {
                                def_id: trait_bound_id,
                                substs: tcx.mk_substs_trait(self_ty, &[]),
                            }.to_predicate();

                            selcx.evaluate_obligation(&traits::Obligation::new(
                                cause,
                                ty::ParamEnv::empty(traits::Reveal::UserFacing),
                                predicate
                            ))
                        });

                        Some(did_match)
                    }
                ) {
                    debug!("evaluate: skipping {:?} due to condition", command);
                    continue
                }
            }
            debug!("evaluate: {:?} succeeded", command);
            if let Some(ref message_) = command.message {
                message = Some(message_.clone());
            }

            if let Some(ref label_) = command.label {
                label = Some(label_.clone());
            }
        }

        OnUnimplementedNote {
            label: label.map(|l| l.format(tcx, trait_ref)),
            message: message.map(|m| m.format(tcx, trait_ref))
        }
    }
}

impl<'a, 'gcx, 'tcx> OnUnimplementedFormatString {
    pub fn try_parse(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                     trait_def_id: DefId,
                     from: InternedString,
                     err_sp: Span)
                     -> Result<Self, ErrorReported>
    {
        let result = OnUnimplementedFormatString(from);
        result.verify(tcx, trait_def_id, err_sp)?;
        Ok(result)
    }

    fn verify(&self,
              tcx: TyCtxt<'a, 'gcx, 'tcx>,
              trait_def_id: DefId,
              span: Span)
              -> Result<(), ErrorReported>
    {
        let name = tcx.item_name(trait_def_id);
        let generics = tcx.generics_of(trait_def_id);
        let parser = Parser::new(&self.0);
        let types = &generics.types;
        let mut result = Ok(());
        for token in parser {
            match token {
                Piece::String(_) => (), // Normal string, no need to check it
                Piece::NextArgument(a) => match a.position {
                    // `{Self}` is allowed
                    Position::ArgumentNamed(s) if s == "Self" => (),
                    // `{ThisTraitsName}` is allowed
                    Position::ArgumentNamed(s) if s == name => (),
                    // So is `{A}` if A is a type parameter
                    Position::ArgumentNamed(s) => match types.iter().find(|t| {
                        t.name == s
                    }) {
                        Some(_) => (),
                        None => {
                            span_err!(tcx.sess, span, E0230,
                                      "there is no type parameter \
                                       {} on trait {}",
                                      s, name);
                            result = Err(ErrorReported);
                        }
                    },
                    // `{:1}` and `{}` are not to be used
                    Position::ArgumentIs(_) => {
                        span_err!(tcx.sess, span, E0231,
                                  "only named substitution \
                                   parameters are allowed");
                        result = Err(ErrorReported);
                    }
                }
            }
        }

        result
    }

    pub fn format(&self,
                  tcx: TyCtxt<'a, 'gcx, 'tcx>,
                  trait_ref: ty::TraitRef<'tcx>)
                  -> String
    {
        let name = tcx.item_name(trait_ref.def_id);
        let trait_str = tcx.item_path_str(trait_ref.def_id);
        let generics = tcx.generics_of(trait_ref.def_id);
        let generic_map = generics.types.iter().map(|param| {
            (param.name.as_str().to_string(),
             trait_ref.substs.type_for_def(param).to_string())
        }).collect::<FxHashMap<String, String>>();

        let parser = Parser::new(&self.0);
        parser.map(|p| {
            match p {
                Piece::String(s) => s,
                Piece::NextArgument(a) => match a.position {
                    Position::ArgumentNamed(s) => match generic_map.get(s) {
                        Some(val) => val,
                        None if s == name => {
                            &trait_str
                        }
                        None => {
                            bug!("broken on_unimplemented {:?} for {:?}: \
                                  no argument matching {:?}",
                                 self.0, trait_ref, s)
                        }
                    },
                    _ => {
                        bug!("broken on_unimplemented {:?} - bad \
                              format arg", self.0)
                    }
                }
            }
        }).collect()
    }
}
