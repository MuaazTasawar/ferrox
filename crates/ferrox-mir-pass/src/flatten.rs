//! Control-flow flattening (Layer 2's core transformation).
//!
//! **Step 8b (this version):** flattening/opaque-predicates (steps
//! 7b-ii-A, 7b-ii-B, 7c) now only apply to functions marked
//! `#[ferrox_macros::protect]` -- detected by scanning the body's `Call`
//! terminators for one whose callee's `def_path_str` ends in
//! `__ferrox_protect_marker` (see `ferrox-runtime::__ferrox_protect_marker`
//! and `ferrox-macros::protect_macro` for the injection side). Previously
//! (through step 7c) every function in the crate was flattened
//! unconditionally -- that was a deliberate Phase 7 smoke-test
//! simplification, not the final design; this step closes that gap.
//!
//! Detection uses `Operand::ty(&body.local_decls, tcx)` ->
//! `TyKind::FnDef(def_id, _)` rather than a more specific
//! "is this operand a constant function reference" helper method, on the
//! theory that this lower-level pattern is less likely to have shifted
//! between nightly releases than a more specific convenience method would
//! be.

use std::collections::BTreeMap;

use rustc_middle::mir::{
    BasicBlock, BasicBlockData, BinOp, Body, Const, ConstOperand, LocalDecl, Operand, Place,
    Rvalue, SourceInfo, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind,
    WithRetag,
};
use rustc_middle::ty::{TyCtxt, TyKind};

/// Step 8b: scans `body`'s `Call` terminators for a call to
/// `__ferrox_protect_marker`. Returns `true` as soon as one is found.
fn is_protected<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> bool {
    for bb_data in body.basic_blocks.iter() {
        if let TerminatorKind::Call { func, .. } = &bb_data.terminator().kind {
            let ty = func.ty(&body.local_decls, tcx);
            if let TyKind::FnDef(def_id, _) = ty.kind() {
                let path = tcx.def_path_str(*def_id);
                if path.ends_with("__ferrox_protect_marker") {
                    return true;
                }
            }
        }
    }
    false
}

/// Builds a `usize` constant operand.
fn make_usize_const_operand<'tcx>(
    tcx: TyCtxt<'tcx>,
    span: rustc_span::Span,
    value: u64,
) -> Operand<'tcx> {
    let const_val = Const::from_usize(tcx, value);
    Operand::Constant(Box::new(ConstOperand {
        span,
        user_ty: None,
        const_: const_val,
    }))
}

/// Builds a `<place> = const <n>` (as a `usize`) statement.
fn make_set_state_stmt<'tcx>(
    span: rustc_span::Span,
    state_place: Place<'tcx>,
    value_operand: Operand<'tcx>,
) -> Statement<'tcx> {
    let rvalue = Rvalue::Use(value_operand, WithRetag::No);
    Statement::new(
        SourceInfo::outermost(span),
        StatementKind::Assign(Box::new((state_place, rvalue))),
    )
}

/// Builds the statements and terminator for an opaque-predicate guarded
/// jump into `real_target`, with `decoy_target` as the (never taken)
/// false arm. Allocates three fresh locals in `body`.
fn make_opaque_guard<'tcx>(
    tcx: TyCtxt<'tcx>,
    span: rustc_span::Span,
    body: &mut Body<'tcx>,
    state_place: Place<'tcx>,
    real_target: BasicBlock,
    decoy_target: BasicBlock,
) -> (Vec<Statement<'tcx>>, TerminatorKind<'tcx>) {
    let usize_ty = tcx.types.usize;
    let bool_ty = tcx.types.bool;

    let sq_local = body.local_decls.push(LocalDecl::new(usize_ty, span));
    let sq_place = Place::from(sq_local);
    let md_local = body.local_decls.push(LocalDecl::new(usize_ty, span));
    let md_place = Place::from(md_local);
    let cond_local = body.local_decls.push(LocalDecl::new(bool_ty, span));
    let cond_place = Place::from(cond_local);

    let four = make_usize_const_operand(tcx, span, 4);
    let two = make_usize_const_operand(tcx, span, 2);

    let sq_stmt = Statement::new(
        SourceInfo::outermost(span),
        StatementKind::Assign(Box::new((
            sq_place,
            Rvalue::BinaryOp(
                BinOp::Mul,
                Box::new((Operand::Copy(state_place), Operand::Copy(state_place))),
            ),
        ))),
    );
    let md_stmt = Statement::new(
        SourceInfo::outermost(span),
        StatementKind::Assign(Box::new((
            md_place,
            Rvalue::BinaryOp(BinOp::Rem, Box::new((Operand::Copy(sq_place), four))),
        ))),
    );
    let cond_stmt = Statement::new(
        SourceInfo::outermost(span),
        StatementKind::Assign(Box::new((
            cond_place,
            Rvalue::BinaryOp(BinOp::Ne, Box::new((Operand::Copy(md_place), two))),
        ))),
    );

    let targets = SwitchTargets::new(std::iter::once((0u128, decoy_target)), real_target);
    let terminator_kind = TerminatorKind::SwitchInt {
        discr: Operand::Move(cond_place),
        targets,
    };

    (vec![sq_stmt, md_stmt, cond_stmt], terminator_kind)
}

/// Builds a basic block with the given statements and terminator kind, not
/// a cleanup/unwind block.
fn make_block<'tcx>(
    span: rustc_span::Span,
    statements: Vec<Statement<'tcx>>,
    kind: TerminatorKind<'tcx>,
) -> BasicBlockData<'tcx> {
    let terminator = Terminator {
        source_info: SourceInfo::outermost(span),
        kind,
        attributes: Default::default(),
    };
    let mut data = BasicBlockData::new(Some(terminator), false);
    data.statements = statements;
    data
}

/// Step 8b: only flattens functions marked `#[ferrox_macros::protect]`
/// (detected via `is_protected`). Unmarked functions are left completely
/// untouched, a change from the Phase 7 blanket-flattening smoke-test
/// behavior.
pub fn maybe_flatten<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
    let debug = std::env::var("FERROX_MIR_DEBUG").as_deref() == Ok("1");

    if !is_protected(tcx, body) {
        if debug {
            eprintln!(
                "ferrox-mir-pass: skipping {:?} (not marked #[protect])",
                tcx.def_path_str(body.source.def_id())
            );
        }
        return;
    }

    let num_original_blocks = body.basic_blocks.len();
    if num_original_blocks <= 1 {
        if debug {
            eprintln!(
                "ferrox-mir-pass: flatten skipped for {:?} (protected, but only {} block(s), nothing to flatten)",
                tcx.def_path_str(body.source.def_id()),
                num_original_blocks
            );
        }
        return;
    }

    let span = body.span;

    let mut switch_targets_needing_pads: Vec<BasicBlock> = Vec::new();
    let mut switch_terminators_touched = 0u32;
    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        if let TerminatorKind::SwitchInt { targets, .. } = &body.basic_blocks[bb].terminator().kind {
            let mut touches_any = false;
            for (_, target) in targets.iter() {
                if target.as_usize() < num_original_blocks {
                    if !switch_targets_needing_pads.contains(&target) {
                        switch_targets_needing_pads.push(target);
                    }
                    touches_any = true;
                }
            }
            let otherwise = targets.otherwise();
            if otherwise.as_usize() < num_original_blocks {
                if !switch_targets_needing_pads.contains(&otherwise) {
                    switch_targets_needing_pads.push(otherwise);
                }
                touches_any = true;
            }
            if touches_any {
                switch_terminators_touched += 1;
            }
        }
    }
    switch_targets_needing_pads.sort_by_key(|bb| bb.as_usize());

    let landing_pad_count = switch_targets_needing_pads.len();
    let landing_pad_base = num_original_blocks;
    let unreachable_bb = BasicBlock::from_usize(landing_pad_base + landing_pad_count);
    let dispatch_bb = BasicBlock::from_usize(landing_pad_base + landing_pad_count + 1);

    let landing_pad_for: BTreeMap<BasicBlock, BasicBlock> = switch_targets_needing_pads
        .iter()
        .enumerate()
        .map(|(offset, &target)| (target, BasicBlock::from_usize(landing_pad_base + offset)))
        .collect();

    let state_local = body.local_decls.push(LocalDecl::new(tcx.types.usize, span));
    let state_place = Place::from(state_local);

    let mut goto_edges_flattened = 0u32;
    let mut opaque_guards_inserted = 0u32;
    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        let terminator_kind = body.basic_blocks[bb].terminator().kind.clone();
        if let TerminatorKind::Goto { target } = terminator_kind {
            if target.as_usize() < num_original_blocks {
                let value_operand = make_usize_const_operand(tcx, span, target.as_u32() as u64);
                let set_stmt = make_set_state_stmt(span, state_place, value_operand);
                let (guard_stmts, guard_kind) =
                    make_opaque_guard(tcx, span, body, state_place, dispatch_bb, unreachable_bb);
                let data = &mut body.basic_blocks_mut()[bb];
                data.statements.push(set_stmt);
                data.statements.extend(guard_stmts);
                data.terminator_mut().kind = guard_kind;
                goto_edges_flattened += 1;
                opaque_guards_inserted += 1;
            }
        }
    }

    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        let terminator_kind = body.basic_blocks[bb].terminator().kind.clone();
        if let TerminatorKind::SwitchInt { discr, targets } = terminator_kind {
            let new_arms: Vec<(u128, BasicBlock)> = targets
                .iter()
                .map(|(value, target)| {
                    let new_target = landing_pad_for.get(&target).copied().unwrap_or(target);
                    (value, new_target)
                })
                .collect();
            let old_otherwise = targets.otherwise();
            let new_otherwise = landing_pad_for.get(&old_otherwise).copied().unwrap_or(old_otherwise);

            let new_targets = SwitchTargets::new(new_arms.into_iter(), new_otherwise);
            body.basic_blocks_mut()[bb].terminator_mut().kind = TerminatorKind::SwitchInt {
                discr,
                targets: new_targets,
            };
        }
    }

    for &target in &switch_targets_needing_pads {
        let value_operand = make_usize_const_operand(tcx, span, target.as_u32() as u64);
        let set_stmt = make_set_state_stmt(span, state_place, value_operand);
        let (guard_stmts, guard_kind) =
            make_opaque_guard(tcx, span, body, state_place, dispatch_bb, unreachable_bb);
        let mut statements = vec![set_stmt];
        statements.extend(guard_stmts);
        let pushed = body
            .basic_blocks_mut()
            .push(make_block(span, statements, guard_kind));
        debug_assert_eq!(
            pushed,
            landing_pad_for[&target],
            "ferrox-mir-pass: landing pad index drifted from its reserved slot"
        );
        opaque_guards_inserted += 1;
    }

    let pushed_unreachable = body
        .basic_blocks_mut()
        .push(make_block(span, vec![], TerminatorKind::Unreachable));
    debug_assert_eq!(pushed_unreachable, unreachable_bb);

    let mut dispatch_arms: Vec<(u128, BasicBlock)> = Vec::with_capacity(num_original_blocks);
    for i in 0..num_original_blocks {
        dispatch_arms.push((i as u128, BasicBlock::from_usize(i)));
    }
    let dispatch_targets = SwitchTargets::new(dispatch_arms.into_iter(), unreachable_bb);
    let pushed_dispatch = body.basic_blocks_mut().push(make_block(
        span,
        vec![],
        TerminatorKind::SwitchInt {
            discr: Operand::Copy(state_place),
            targets: dispatch_targets,
        },
    ));
    debug_assert_eq!(pushed_dispatch, dispatch_bb);

    if debug {
        eprintln!(
            "ferrox-mir-pass: flattened {:?} (protected) -- {} original block(s), {} Goto edge(s), {} SwitchInt terminator(s) touched, {} landing pad(s), {} opaque guard(s) inserted",
            tcx.def_path_str(body.source.def_id()),
            num_original_blocks,
            goto_edges_flattened,
            switch_terminators_touched,
            landing_pad_count,
            opaque_guards_inserted
        );
    }
}
