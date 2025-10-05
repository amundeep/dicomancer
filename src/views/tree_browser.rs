use crate::components::segmented_toggle::tree_view_mode_toggle;
use crate::message::Message;
use crate::model::{DicomEntry, TreeNodeKey, TreeViewMode};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, row, text, Column, Space};
use iced::Length;
use std::collections::{BTreeMap, BTreeSet};

const INDENT: f32 = 18.0;

pub fn tree_panel<'a>(
    entries: &'a [DicomEntry],
    tree_view_mode: TreeViewMode,
    collapsed_nodes: &BTreeSet<TreeNodeKey>,
    selected_instance: Option<usize>,
) -> Column<'a, Message> {
    let mut root = column![text("Imported Instances").size(20)];

    let toggle_row = tree_view_mode_toggle(tree_view_mode);
    root = root.push(toggle_row);

    if entries.is_empty() {
        return root.push(text("No files imported"));
    }

    match tree_view_mode {
        TreeViewMode::FileBrowser => build_file_list(root, entries, selected_instance),
        TreeViewMode::UidTree => build_uid_tree(root, entries, collapsed_nodes, selected_instance),
    }
    .spacing(6)
}

fn build_file_list<'a>(
    base: Column<'a, Message>,
    entries: &'a [DicomEntry],
    selected_instance: Option<usize>,
) -> Column<'a, Message> {
    entries
        .iter()
        .enumerate()
        .fold(base, |column, (index, entry)| {
            let is_selected = selected_instance == Some(index);
            let path_text = entry.view.file_path.display().to_string();
            let button_label = if is_selected {
                format!("▶ {path_text}")
            } else {
                path_text
            };

            column.push(
                button(
                    text(button_label)
                        .wrapping(Wrapping::Word)
                        .width(Length::Fill),
                )
                .on_press(Message::SelectInstance(index)),
            )
        })
}

type SopIndexList = Vec<usize>;
type SopMap<'a> = BTreeMap<&'a str, SopIndexList>;
type SeriesMap<'a> = BTreeMap<&'a str, SopMap<'a>>;
type StudyMap<'a> = BTreeMap<&'a str, SeriesMap<'a>>;
type GroupedTree<'a> = BTreeMap<&'a str, StudyMap<'a>>;

fn build_uid_tree<'a>(
    base: Column<'a, Message>,
    entries: &'a [DicomEntry],
    collapsed_nodes: &BTreeSet<TreeNodeKey>,
    selected_instance: Option<usize>,
) -> Column<'a, Message> {
    let mut grouped: GroupedTree = BTreeMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        let patient_map = grouped.entry(entry.patient_id.as_str()).or_default();
        let study_map = patient_map
            .entry(entry.study_instance_uid.as_str())
            .or_default();
        let series_map = study_map
            .entry(entry.series_instance_uid.as_str())
            .or_default();
        series_map
            .entry(entry.sop_instance_uid.as_str())
            .or_default()
            .push(idx);
    }

    let arrow = |collapsed: bool| if collapsed { "▶" } else { "▼" };

    grouped
        .into_iter()
        .fold(base, |column, (patient_id, studies)| {
            let patient_key = TreeNodeKey::patient(patient_id);
            let patient_collapsed = collapsed_nodes.contains(&patient_key);
            let patient_label = format!("{} PatientID: {patient_id}", arrow(patient_collapsed));
            let mut column =
                column
                    .push(row![button(text(patient_label))
                        .on_press(Message::ToggleNode(patient_key.clone())),]);

            if patient_collapsed {
                return column;
            }

            for (study_uid, series_map) in studies {
                let study_key = TreeNodeKey::study(patient_id, study_uid);
                let study_collapsed = collapsed_nodes.contains(&study_key);
                let study_label =
                    format!("{} StudyInstanceUID: {study_uid}", arrow(study_collapsed));
                column = column.push(row![
                    Space::with_width(Length::Fixed(INDENT)),
                    button(text(study_label)).on_press(Message::ToggleNode(study_key.clone())),
                ]);

                if study_collapsed {
                    continue;
                }

                for (series_uid, sop_map) in series_map {
                    let series_key = TreeNodeKey::series(patient_id, study_uid, series_uid);
                    let series_collapsed = collapsed_nodes.contains(&series_key);
                    let series_label = format!(
                        "{} SeriesInstanceUID: {series_uid}",
                        arrow(series_collapsed)
                    );
                    column = column.push(row![
                        Space::with_width(Length::Fixed(INDENT * 2.0)),
                        button(text(series_label))
                            .on_press(Message::ToggleNode(series_key.clone())),
                    ]);

                    if series_collapsed {
                        continue;
                    }

                    for (sop_uid, indices) in sop_map {
                        for index in indices {
                            let label = format!("SOPInstanceUID: {sop_uid}");
                            let is_selected = selected_instance == Some(index);
                            let button_label = if is_selected {
                                format!("▶ {label}")
                            } else {
                                label
                            };
                            column = column.push(row![
                                Space::with_width(Length::Fixed(INDENT * 3.0)),
                                button(text(button_label)).on_press(Message::SelectInstance(index)),
                            ]);
                        }
                    }
                }
            }

            column
        })
}
