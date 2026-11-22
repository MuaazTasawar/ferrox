//! Control-flow flattening (Layer 2's core transformation).
//!
//! **Step 7b-ii-A (this version):** flattens `Goto` edges only. For every
//! original basic block whose terminator is `Goto { target }` where
//! `target` is one of the function's original blocks, this appends a
//! `state = <target's index>` statement to the block and retargets its
//! `Goto` at a new dispatch block, which `SwitchInt`s on `state` back out
//! to the real target. `SwitchInt`, `Call`, `Drop`, `Assert`, and all
//! other terminator kinds are left completely untouched in this step --
//! deferred to step 7b-ii-B (`SwitchInt`) and documented as an ongoing
//! scope limitation for the rest.
//!
//! API corrections against this nightly's real MIR types (see compiler
//! errors in chat history): `Rvalue::Use` now takes a second `WithRetag`
//! argument (`WithRetag::No` here -- this synthetic assignment needs no
//! Stacked-Borrows retag); `Statement` is `#[non_exhaustive]`, built via
//! `Statement::new(source_info, kind)` rather than a struct literal;
//! `Terminator` gained an `attributes` field (`Default::default()` here);
//! `BasicBlockData::new` takes an added `is_cleanup: bool` parameter
//! (`false` for both new blocks here -- neither is a cleanup/unwind block).

use rustc_middle::mir::{
    BasicBlock, BasicBlockData, Body, Const, ConstOperand, LocalDecl, Operand, Place, Rvalue,
    SourceInfo, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind, WithRetag,
};
use rustc_middle::ty::TyCtxt;

/// Builds a `<place> = const <n>` (as a `usize`) statement.
fn make_set_state_stmt<'tcx>(
    tcx: TyCtxt<'tcx>,
    span: rustc_span::Span,
    state_place: Place<'tcx>,
    value: u32,
) -> Statement<'tcx> {
    let const_val = Const::from_usize(tcx, value as u64);
    let const_operand = ConstOperand {
        span,
        user_ty: None,
        const_: const_val,
    };
    let rvalue = Rvalue::Use(Operand::Constant(Box::new(const_operand)), WithRetag::No);
    Statement::new(
        SourceInfo::outermost(span),
        StatementKind::Assign(Box::new((state_place, rvalue))),
    )
}

/// Builds a terminator-only basic block (no statements), not a cleanup block.
fn make_terminator_only_block<'tcx>(
    span: rustc_span::Span,
    kind: TerminatorKind<'tcx>,
) -> BasicBlockData<'tcx> {
    let terminator = Terminator {
        source_info: SourceInfo::outermost(span),
        kind,
        attributes: Default::default(),
    };
    BasicBlockData::new(Some(terminator), false)
}

/// Step 7b-ii-A: flattens `Goto` edges in `body`, in place.
pub fn maybe_flatten<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
    let debug = std::env::var("FERROX_MIR_DEBUG").as_deref() == Ok("1");

    let num_original_blocks = body.basic_blocks.len();
    if num_original_blocks <= 1 {
        if debug {
            eprintln!(
                "ferrox-mir-pass: flatten skipped for {:?} ({} block(s), nothing to flatten)",
                tcx.def_path_str(body.source.def_id()),
                num_original_blocks
            );
        }
        return;
    }

    let span = body.span;
    let state_local = body.local_decls.push(LocalDecl::new(tcx.types.usize, span));
    let state_place = Place::from(state_local);

    // The dispatch block will be pushed as the *second* new block below
    // (after the unreachable block), so its index is
    // `num_original_blocks + 1`. We compute it up front so the Goto
    // retargeting loop below can reference it before it exists.
    let dispatch_bb = BasicBlock::from_usize(num_original_blocks + 1);

    let mut goto_edges_flattened = 0u32;

    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        let terminator_kind = body.basic_blocks[bb].terminator().kind.clone();

        if let TerminatorKind::Goto { target } = terminator_kind {
            if target.as_usize() < num_original_blocks {
                let target_idx = target.as_u32();
                let set_stmt = make_set_state_stmt(tcx, span, state_place, target_idx);
                let data = &mut body.basic_blocks_mut()[bb];
                data.statements.push(set_stmt);
                data.terminator_mut().kind = TerminatorKind::Goto { target: dispatch_bb };
                goto_edges_flattened += 1;
            }
        }
    }

    // Unreachable fallback block (index num_original_blocks).
    let unreachable_bb = body
        .basic_blocks_mut()
        .push(make_terminator_only_block(span, TerminatorKind::Unreachable));

    // Dispatch block (index num_original_blocks + 1, matching dispatch_bb above).
    let mut dispatch_arms: Vec<(u128, BasicBlock)> = Vec::with_capacity(num_original_blocks);
    for i in 0..num_original_blocks {
        dispatch_arms.push((i as u128, BasicBlock::from_usize(i)));
    }
    let dispatch_targets = SwitchTargets::new(dispatch_arms.into_iter(), unreachable_bb);
    let actual_dispatch_bb = body.basic_blocks_mut().push(make_terminator_only_block(
        span,
        TerminatorKind::SwitchInt {
            discr: Operand::Copy(state_place),
            targets: dispatch_targets,
        },
    ));

    debug_assert_eq!(
        actual_dispatch_bb, dispatch_bb,
        "ferrox-mir-pass: dispatch block index drifted from its reserved slot"
    );

    if debug {
        eprintln!(
            "ferrox-mir-pass: flattened {:?} -- {} original block(s), {} Goto edge(s) flattened (step 7b-ii-A: Goto only)",
            tcx.def_path_str(body.source.def_id()),
            num_original_blocks,
            goto_edges_flattened
        );
    }
}
