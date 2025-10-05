#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeNodeKey {
    Patient(String),
    Study {
        patient: String,
        study: String,
    },
    Series {
        patient: String,
        study: String,
        series: String,
    },
}

impl TreeNodeKey {
    pub fn patient(id: &str) -> Self {
        Self::Patient(id.to_string())
    }

    pub fn study(patient: &str, study: &str) -> Self {
        Self::Study {
            patient: patient.to_string(),
            study: study.to_string(),
        }
    }

    pub fn series(patient: &str, study: &str, series: &str) -> Self {
        Self::Series {
            patient: patient.to_string(),
            study: study.to_string(),
            series: series.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeViewMode {
    #[default]
    FileBrowser,
    UidTree,
}
