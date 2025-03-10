use biome_control_flow::builder::BlockId;
use biome_js_syntax::{JsElseClause, JsIfStatement};
use biome_rowan::{AstNode, SyntaxResult};

use crate::services::control_flow::{
    FunctionBuilder,
    visitor::{NodeVisitor, StatementStack},
};

pub(in crate::services::control_flow) struct IfVisitor {
    /// Saved position of the control flow cursor before entering the statement
    entry_block: BlockId,
    /// First block of the consequent for this statement
    consequent_start: BlockId,
    /// Last block of the consequent for this statement, implicitly jumps to
    /// the next block
    /// This is only set if the statement has an else clause (by the [ElseVisitor]),
    /// otherwise this information is derived from the cursor of the function builder
    consequent_end: Option<BlockId>,
    /// Start and end block of the alternate of this statement, set by the
    /// [ElseVisitor] if this node has an else clause
    alt_block: Option<(BlockId, BlockId)>,
}

impl NodeVisitor for IfVisitor {
    type Node = JsIfStatement;

    fn enter(
        _: Self::Node,
        builder: &mut FunctionBuilder,
        _: StatementStack,
    ) -> SyntaxResult<Self> {
        let entry_block = builder.cursor();

        let consequent_start = builder.append_block();
        builder.set_cursor(consequent_start);

        Ok(Self {
            entry_block,
            consequent_start,
            consequent_end: None,
            alt_block: None,
        })
    }

    fn exit(
        self,
        node: Self::Node,
        builder: &mut FunctionBuilder,
        _: StatementStack,
    ) -> SyntaxResult<()> {
        let consequent_end = self.consequent_end.unwrap_or_else(|| builder.cursor());

        let alt_block = self.alt_block;

        let next_block = builder.append_block();

        builder.set_cursor(self.entry_block);

        // If this node has an else clause the start and end of `alt_block`
        // were generated by the `ElseVisitor`
        if let Some((alt_start, alt_end)) = alt_block {
            builder
                .append_jump(true, self.consequent_start)
                .with_node(node.test()?.into_syntax());
            builder.append_jump(false, alt_start);

            builder.set_cursor(alt_end);
            builder.append_jump(false, next_block);
        } else {
            builder
                .append_jump(true, self.consequent_start)
                .with_node(node.test()?.into_syntax());
            builder.append_jump(false, next_block);
        }

        builder.set_cursor(consequent_end);
        builder.append_jump(false, next_block);

        builder.set_cursor(next_block);

        Ok(())
    }
}

pub(in crate::services::control_flow) struct ElseVisitor {
    consequent_block: BlockId,
    alt_block: BlockId,
}

impl NodeVisitor for ElseVisitor {
    type Node = JsElseClause;

    fn enter(
        _: Self::Node,
        builder: &mut FunctionBuilder,
        _: StatementStack,
    ) -> SyntaxResult<Self> {
        let consequent_block = builder.cursor();

        let alt_block = builder.append_block();
        builder.set_cursor(alt_block);

        Ok(Self {
            consequent_block,
            alt_block,
        })
    }

    fn exit(
        self,
        _: Self::Node,
        builder: &mut FunctionBuilder,
        stack: StatementStack,
    ) -> SyntaxResult<()> {
        let if_state = stack.read_top::<IfVisitor>()?;

        if_state.consequent_end = Some(self.consequent_block);
        if_state.alt_block = Some((self.alt_block, builder.cursor()));

        Ok(())
    }
}
