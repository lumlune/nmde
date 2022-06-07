use {
    crate::io::nmd::anatomy::NmdFileBoneFlag,
    crate::ui::region::NodeView,
    std::collections::{HashMap, HashSet},
    std::path::PathBuf,
    eframe::egui::Color32,
};

/* TODO:
 * ~ Move/rename this module, couple it with `io::fifo::message`?
 */

#[derive(Debug)]
pub enum UiComponent {
    MenuCommit,
    MenuImport(PathBuf), // Later: paramaterize to IV, V, VI
    MenuExport(PathBuf), // Later: paramaterize to IV, V, VI
    MenuHideListIds(bool),
    MenuProjectOpen(PathBuf),
    MenuProjectSaveAs(PathBuf),
    MenuTab(usize),
    MenuTabClose(usize),
    TreeFilterClear,
    TreeNode(u16, String),
    TreeNodeCopy(u16, String),
    TreeNodeCopySingle(u16, String),
    TreeNodeCut(u16, String),
    TreeNodeDelete(u16),
    TreeNodePaste(u16),
    TreeNodePasteAfter(u16),
    TreeNodeExpansion(u16),
    TreeNodeFilterTo(u16),
    TreeNodeRoot(u16),
    TreeNodeUnroot,
    TreeNodeUnrootAll,
    TreeNodePin(u16, String),
    TreeNodePinRemove(u16),
    TreeNodeScroll(u16),
    TreeNodeFocus(u16, String),
    TreeNodeSpotlight(u16, String),
    TreeNodeView(NodeView),
}

// Reserve for things that HAVE changed, not ought to
#[derive(Debug)]
pub enum UiState {
    BoneData(u16, bool),
    BoneName(u16, String),
    BoneFlag(u16, NmdFileBoneFlag),
    TreeEditStatus(bool),
    TreeNodeCopyPaste(HashMap<u16, (u16, String)>, u16),
    TreeNodeCutPaste(u16, u16),
    TreeNodeDelete(u16, HashSet<u16>),
    TreeNodeScrollDone,
    TreeNodeViewChanged(NodeView),
}
