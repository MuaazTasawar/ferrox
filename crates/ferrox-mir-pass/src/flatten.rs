//! Control-flow flattening (Layer 2's core transformation).
//!
//! **Step 7b-ii-A + 7b-ii-B (this version):** flattens both `Goto` edges
//! and `SwitchInt` terminators whose targets are original blocks.
//! - `Goto { target }`: the state-setting statement is appended directly
//!   to the source block (no extra block needed), which then jumps to the
//!   shared dispatch block instead of `target` directly.
//! - `SwitchInt { discr, targets }`: cannot hold its own statements, so
//!   each distinct original-block target referenced by any arm (or
//!   `otherwise`) gets a deduplicated "landing pad" block (`state =
//!   target; goto dispatch`), and the switch's arms are rewritten to
//!   point at the landing pad instead of the real target. The
//!   discriminant computation itself (`discr`) is left untouched -- this
//!   step obscures the block *graph*, not which arm's condition is true;
//!   obscuring the condition itself is opaque predicates' job (step 7c).
//!
//! `Call`, `Drop`, `Assert`, `Return`, and unwind targets are still left
//! completely untouched -- an ongoing, documented scope limitation, not
//! addressed by this step.

use std::collections::BTreeMap;

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

/// Step 7b-ii-A + 7b-ii-B: flattens `Goto` edges and `SwitchInt`
/// terminators in `body`, in place.
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

    // Pass 1 (read-only): find every distinct original-block target
    // referenced by any SwitchInt arm or otherwise, across all original
    // blocks. Each one needs exactly one landing pad, deduplicated.
    let mut switch_targets_needing_pads: Vec<BasicBlock> = Vec::new();
    let mut switch_terminators_touched = 0u32;
    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        if let TerminatorKind::SwitchInt { targets, .. } = &body.basic_blocks[bb].terminator().kind {
            let mut touches_any = false;
            for (_, target) in targets.iter() {
                if target.as_usize() < num_original_blocks
                    && !switch_targets_needing_pads.contains(&target)
                {
                    switch_targets_needing_pads.push(target);
                    touches_any = true;
                } else if target.as_usize() < num_original_blocks {
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
    // Sorted, stable order so the index math below is deterministic.
    switch_targets_needing_pads.sort_by_key(|bb| bb.as_usize());

    let landing_pad_count = switch_targets_needing_pads.len();
    // Layout: [original blocks] [landing pads] [unreachable] [dispatch]
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

    // Pass 2: flatten Goto edges (unchanged approach from step 7b-ii-A,
    // now targeting the recomputed dispatch_bb).
    let mut goto_edges_flattened = 0u32;
    for i in 0..num_original_blocks {
        let bb = BasicBlock::from_usize(i);
        let terminator_kind = body.basic_blocks[bb].terminator().kind.clone();
        if let TerminatorKind::Goto { target } = terminator_kind {
            if target.as_usize() < num_original_blocks {
                let set_stmt = make_set_state_stmt(tcx, span, state_place, target.as_u32());
                let data = &mut body.basic_blocks_mut()[bb];
                data.statements.push(set_stmt);
                data.terminator_mut().kind = TerminatorKind::Goto { target: dispatch_bb };
                goto_edges_flattened += 1;
            }
        }
    }

    // Pass 3: rewrite SwitchInt terminators to point at landing pads
    // instead of real targets directly.
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

    // Pass 4: materialize landing pad blocks, in the same sorted order
    // used to compute their indices above.
    for &target in &switch_targets_needing_pads {
        let set_stmt = make_set_state_stmt(tcx, span, state_place, target.as_u32());
        let pushed = body
            .basic_blocks_mut()
            .push(make_block(span, vec![set_stmt], TerminatorKind::Goto { target: dispatch_bb }));
        debug_assert_eq!(
            pushed,
            landing_pad_for[&target],
            "ferrox-mir-pass: landing pad index drifted from its reserved slot"
        );
    }

    // Pass 5: unreachable fallback + dispatch block.
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
            "ferrox-mir-pass: flattened {:?} -- {} original block(s), {} Goto edge(s), {} SwitchInt terminator(s) touched, {} landing pad(s) created (step 7b-ii-A+B)",
            tcx.def_path_str(body.source.def_id()),
            num_original_blocks,
            goto_edges_flattened,
            switch_terminators_touched,
            landing_pad_count
        );
    }
}
