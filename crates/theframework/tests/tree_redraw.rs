#![cfg(feature = "ui")]
use theframework::prelude::*;

#[test]
fn tree_repaints_visible_changes_but_ignores_closed_descendants() {
    for scale in [1.0, 2.0] {
        let mut ctx = TheContext::new(320, 160, scale);
        ctx.ui_render_scale = scale;
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 160));
        buffer.set_render_scale(scale);
        let mut tree = TheTreeLayout::new(TheId::named("Tree"));
        let branch_id = TheId::named("Branch");
        let hidden_id = TheId::named("Hidden");
        let mut branch = TheTreeNode::new(branch_id.clone());
        branch.add_child(TheTreeNode::new(hidden_id.clone()));
        tree.get_root().add_child(branch);
        tree.set_dim(TheDim::sized(320, 160), &mut ctx);
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(
            !tree.needs_redraw(),
            "closed children must not keep the tree dirty"
        );
        tree.get_node_by_id_mut(&hidden_id.uuid)
            .unwrap()
            .widget
            .set_state(TheWidgetState::Selected);
        assert!(!tree.needs_redraw());
        let collapsed = buffer.pixels().to_vec();
        tree.tree_node_state_changed(branch_id.clone(), true);
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(!tree.needs_redraw());
        assert_ne!(collapsed, buffer.pixels());
        let expanded = buffer.pixels().to_vec();
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert_eq!(
            expanded,
            buffer.pixels(),
            "cached and forced redraw must agree"
        );
        tree.tree_node_state_changed(branch_id, false);
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert_eq!(collapsed, buffer.pixels());
        assert!(!tree.needs_redraw());
        tree.set_background_color(None);
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(!tree.needs_redraw());
        for i in 0..20 {
            tree.get_root()
                .add_child(TheTreeNode::new(TheId::named(&format!("Row {i}"))));
        }
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(!tree.needs_redraw());
        let before_scroll = buffer.pixels().to_vec();
        tree.scroll_by(Vec2::new(0, -60));
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert_ne!(before_scroll, buffer.pixels());
        assert!(!tree.needs_redraw());
        tree.get_root().clear();
        assert!(
            tree.needs_redraw(),
            "removing the last contents must invalidate the cache"
        );
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(!tree.needs_redraw());
        tree.set_headerless(false);
        assert!(tree.needs_redraw());
        tree.draw(&mut buffer, &mut style, &mut ctx);
        assert!(!tree.needs_redraw());
    }
}
