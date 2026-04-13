use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateField {
    Due,
    Scheduled,
    Starting,
    CreationDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextField {
    Project,
    Context,
    Description,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Due,
    Scheduled,
    Starting,
    CreationDate,
    Priority,
    Project,
    Context,
    Description,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriorityOp {
    Eq,
    Above,
    Below,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOp {
    Includes,
    Excludes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    DateComparison {
        field: DateField,
        op: CompareOp,
        offset: i32,
    },
    PriorityComparison {
        op: PriorityOp,
        letter: char,
    },
    TextMatch {
        field: TextField,
        op: TextOp,
        text: String,
    },
    Existence {
        field: Field,
        present: bool,
    },
    DoneFilter {
        done: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterBlock {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    pub field: Field,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartList {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub group_path: Vec<String>,
    pub source_path: PathBuf,
    pub parse_error: Option<String>,
    pub blocks: Vec<FilterBlock>,
    pub sort_directives: Vec<Directive>,
    pub group_directives: Vec<Directive>,
}
