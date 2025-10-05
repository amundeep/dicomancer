use crate::model::{DicomEntry, TreeNodeKey, TreeViewMode};

#[derive(Debug, Clone)]
pub enum Message {
    PickFiles,
    FilesLoaded(Vec<Result<DicomEntry, String>>),
    SelectInstance(usize),
    ToggleNode(TreeNodeKey),
    SetTreeViewMode(TreeViewMode),
}
